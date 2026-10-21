use axum::{
    Json, Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use ironwood_protocol::*;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub mod archive;
pub mod analysis_runs;
mod archive_http;
mod archive_library;
mod archive_pgn;
pub mod config;
pub mod engine;
pub mod explorer;
pub mod registration;
pub mod updates;
mod tablebases;
pub use config::Config;

struct PendingTicket {
    origin: String,
    expires: Instant,
}
#[derive(Clone)]
struct Shared {
    config: Config,
    tickets: Arc<Mutex<HashMap<String, PendingTicket>>>,
    sockets: Arc<tokio::sync::Semaphore>,
    control: Control,
    rate: Arc<Mutex<HashMap<(std::net::IpAddr, RequestBudget), (Instant, u32)>>>,
    engines: Arc<tokio::sync::Semaphore>,
    registry: registration::Registry,
    archive: Option<archive::Archive>,
    tablebases: tablebases::Service,
}

#[derive(Clone)]
pub struct Control {
    pub updates: updates::Manager,
    engines: Arc<tokio::sync::Semaphore>,
    max_engines: usize,
    stop: tokio::sync::watch::Sender<bool>,
    sockets: Arc<tokio::sync::Semaphore>,
    max_connections: u32,
    background: Arc<tokio::sync::Semaphore>,
    tablebases: tablebases::Service,
    registrations: Arc<tokio::sync::Semaphore>,
}

impl Control {
    /// Hold every engine slot and the registration slot until shutdown starts.
    /// This closes the idle-check/new-engine race without interrupting a session.
    pub fn reserve_update(&self) -> Option<(tokio::sync::OwnedSemaphorePermit, tokio::sync::OwnedSemaphorePermit)> {
        if self.stopping() { return None; }
        let engines = self.engines.clone().try_acquire_many_owned(self.max_engines as u32).ok()?;
        let registration = self.registrations.clone().try_acquire_owned().ok()?;
        Some((engines, registration))
    }
    pub fn active_connections(&self) -> u32 { self.max_connections.saturating_sub(self.sockets.available_permits() as u32) }
    pub fn active_engines(&self) -> usize { self.max_engines.saturating_sub(self.engines.available_permits()) }
    pub fn shutdown(&self) {
        self.stop.send_replace(true);
    }
    pub async fn cancelled(&self) {
        let mut receiver = self.stop.subscribe();
        if *receiver.borrow_and_update() {
            return;
        }
        let _ = receiver.changed().await;
    }
    pub async fn drained(&self) {
        let _permit = self
            .sockets
            .clone()
            .acquire_many_owned(self.max_connections)
            .await;
        let _background = self.background.clone().acquire_owned().await;
        self.tablebases.drained().await;
        let _registrations = self.registrations.clone().acquire_owned().await;
    }
    pub fn stopping(&self) -> bool {
        *self.stop.borrow()
    }
}

pub fn log_event(event: &str, fields: serde_json::Value) {
    // Call sites supply fixed field names and non-sensitive values only.
    println!(
        "{}",
        serde_json::json!({ "event": event, "fields": fields })
    );
}

pub fn app(config: Config) -> Result<Router, Box<dyn std::error::Error>> {
    Ok(app_with_control(config)?.0)
}

pub fn app_with_control(config: Config) -> Result<(Router, Control), Box<dyn std::error::Error>> {
    app_with_registration(config, None)
}

pub fn app_with_registration(
    config: Config,
    path: Option<std::path::PathBuf>,
) -> Result<(Router, Control), Box<dyn std::error::Error>> {
    config.validate()?;
    let sockets = Arc::new(tokio::sync::Semaphore::new(
        config.limits.max_connections as usize,
    ));
    let tablebases = tablebases::Service::new(config.tablebase_directories.clone());
    let engines = Arc::new(tokio::sync::Semaphore::new(config.engine_limits.max_processes));
    let control = Control {
        updates: updates::Manager::default(),
        engines: engines.clone(), max_engines: config.engine_limits.max_processes,
        registrations: Arc::new(tokio::sync::Semaphore::new(1)),
        tablebases: tablebases.clone(),
        stop: tokio::sync::watch::channel(false).0,
        sockets: sockets.clone(),
        max_connections: config.limits.max_connections,
        background: Arc::new(tokio::sync::Semaphore::new(1)),
    };
    let shared = Shared {
        registry: registration::Registry::new(config.engines.clone(), path),
        tablebases,
        archive: config
            .archive
            .as_deref()
            .map(archive::Archive::open)
            .transpose()?,
        engines,
        config,
        tickets: Arc::default(),
        sockets,
        control: control.clone(),
        rate: Arc::default(),
    };
    if let Some(archive) = &shared.archive {
        explorer::spawn(archive.clone(), control.clone());
    }
    let router = Router::new()
        .route(
            "/healthz",
            get(|| async { Json(serde_json::json!({"status":"ok"})) }),
        )
        .route("/readyz", get(readiness))
        .route("/api/v1/capabilities", get(capabilities))
        .route("/api/v1/engines", get(engine_list))
        .route("/api/v1/tickets", post(ticket))
        .route("/api/v1/socket", get(socket))
        .route("/api/v1/shutdown", post(shutdown))
        .route("/api/v1/updates", get(update_status))
        .route("/api/v1/updates/check", post(update_check))
        .route("/api/v1/updates/install", post(update_install))
        .merge(registration::routes())
        .merge(archive_http::routes())
        .merge(tablebases::routes())
        .layer(axum::extract::DefaultBodyLimit::max(archive::MAX_BODY))
        .fallback(|| async { failure(StatusCode::NOT_FOUND, "not_found", "Unknown endpoint") })
        .layer(middleware::from_fn_with_state(shared.clone(), guard))
        .with_state(shared);
    Ok((router, control))
}

async fn readiness(State(shared): State<Shared>) -> Response {
    if shared.control.stopping() {
        failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "shutting_down",
            "Companion is stopping",
        )
    } else {
        Json(serde_json::json!({"status":"ready"})).into_response()
    }
}

async fn shutdown(State(shared): State<Shared>) -> Response {
    shared.control.shutdown();
    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({"status":"stopping"})),
    )
        .into_response()
}
async fn update_status(State(shared): State<Shared>) -> Response {
    Json(shared.control.updates.status(&shared.control)).into_response()
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateCommand {}
async fn update_check(State(shared): State<Shared>, command: Result<Json<UpdateCommand>, axum::extract::rejection::JsonRejection>) -> Response {
    if command.is_err() { return failure(StatusCode::BAD_REQUEST,"invalid_request","Update commands require an empty JSON object"); }
    match shared.control.updates.check() {
        Ok(()) => (StatusCode::ACCEPTED,Json(shared.control.updates.status(&shared.control))).into_response(),
        Err(message) => failure(StatusCode::CONFLICT,"update_unavailable",message),
    }
}
async fn update_install(State(shared): State<Shared>, command: Result<Json<UpdateCommand>, axum::extract::rejection::JsonRejection>) -> Response {
    if command.is_err() { return failure(StatusCode::BAD_REQUEST,"invalid_request","Update commands require an empty JSON object"); }
    match shared.control.updates.install(&shared.control) {
        Ok(()) => (StatusCode::ACCEPTED,Json(shared.control.updates.status(&shared.control))).into_response(),
        Err(message) => failure(StatusCode::CONFLICT,"update_unavailable",message),
    }
}

fn failure(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(ApiError {
            code: code.into(),
            message: message.into(),
        }),
    )
        .into_response()
}

async fn guard(
    State(shared): State<Shared>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let probe = matches!(request.uri().path(), "/healthz" | "/readyz");
    if probe {
        return next.run(request).await;
    }
    let peer = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|p| p.0.ip())
        .unwrap_or(std::net::Ipv4Addr::LOCALHOST.into());
    let budget = request_budget(request.method(), request.uri().path());
    let rate_allowed = allow_request(&shared, peer, budget, started);
    let origin = request
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !shared.config.allowed_origins.iter().any(|o| o == origin) {
        return failure(
            StatusCode::FORBIDDEN,
            "origin_denied",
            "Origin is not allowed",
        );
    }
    let origin = HeaderValue::from_str(origin).unwrap();
    let mut response = if rate_allowed.is_err() {
        failure(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Request budget exceeded; retry in one minute",
        )
    } else if request.method() == Method::OPTIONS {
        let headers = request.headers();
        let method_ok = headers
            .get("access-control-request-method")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| matches!(v, "GET" | "POST"));
        let headers_ok = headers
            .get("access-control-request-headers")
            .and_then(|v| v.to_str().ok())
            .is_none_or(|v| {
                v.split(',').all(|h| {
                    matches!(
                        h.trim().to_ascii_lowercase().as_str(),
                        "authorization" | "content-type" | "x-ironwood-protocol"
                    )
                })
            });
        if method_ok && headers_ok {
            StatusCode::NO_CONTENT.into_response()
        } else {
            failure(
                StatusCode::FORBIDDEN,
                "preflight_denied",
                "Unsupported preflight",
            )
        }
    } else if shared.control.stopping() {
        failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "shutting_down",
            "Companion is stopping",
        )
    } else if request.uri().path() == "/api/v1/socket" {
        bounded_request(request, next, &shared.config.limits).await // Ticket authenticates first frame.
    } else if request
        .headers()
        .get(PROTOCOL_HEADER)
        .and_then(|h| h.to_str().ok())
        != Some(PROTOCOL_VERSION)
    {
        failure(
            StatusCode::UPGRADE_REQUIRED,
            "protocol_mismatch",
            "Expected protocol version 1",
        )
    } else if !authorized(request.headers(), &shared.config.token) {
        failure(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Owner credential required",
        )
    } else {
        bounded_request(request, next, &shared.config.limits).await
    };
    // Normalize framework rejections without logging their potentially sensitive bodies.
    response = match response.status() {
        StatusCode::METHOD_NOT_ALLOWED => failure(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "Unsupported method",
        ),
        StatusCode::BAD_REQUEST
            if response
                .headers()
                .get("content-type")
                .is_none_or(|v| v != "application/json") =>
        {
            failure(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "Malformed request",
            )
        }
        _ => response,
    };
    log_event(
        "request",
        serde_json::json!({"status":response.status().as_u16(),"elapsed_ms":started.elapsed().as_millis()}),
    );
    let headers = response.headers_mut();
    headers.insert("access-control-allow-origin", origin);
    headers.insert("vary", HeaderValue::from_static("Origin"));
    headers.insert(
        "access-control-allow-methods",
        HeaderValue::from_static("GET, POST"),
    );
    headers.insert(
        "access-control-allow-headers",
        HeaderValue::from_static("Authorization, Content-Type, X-Ironwood-Protocol"),
    );
    headers.insert("access-control-expose-headers", HeaderValue::from_static("Retry-After"));
    if let Err(seconds) = rate_allowed {
        headers.insert("retry-after", HeaderValue::from_str(&seconds.to_string()).unwrap());
    }
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RequestBudget {
    Operations,
    ArchiveReads,
    Preflight,
}

fn request_budget(method: &Method, path: &str) -> RequestBudget {
    if method == Method::OPTIONS {
        RequestBudget::Preflight
    } else if (method == Method::GET && path.starts_with("/api/v1/archive/")
        && path != "/api/v1/archive/backup")
        || (method == Method::POST && path == "/api/v1/archive/export")
    {
        RequestBudget::ArchiveReads
    } else {
        // Backups, writes, engine registration and tickets keep the operations budget.
        RequestBudget::Operations
    }
}

fn allow_request(
    shared: &Shared,
    peer: std::net::IpAddr,
    budget: RequestBudget,
    now: Instant,
) -> Result<(), u64> {
    let limit = match budget {
        RequestBudget::Operations => shared.config.limits.requests_per_minute,
        RequestBudget::ArchiveReads => shared.config.limits.archive_reads_per_minute,
        RequestBudget::Preflight => shared.config.limits.preflight_requests_per_minute,
    };
    let mut entries = shared.rate.lock().unwrap();
    entries.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
    let key = (peer, budget);
    // At most 1024 clients per budget; read traffic cannot evict operations capacity.
    if !entries.contains_key(&key)
        && entries.keys().filter(|(_, existing)| *existing == budget).count() >= 1024
    {
        return Err(60);
    }
    let (start, count) = entries.entry(key).or_insert((now, 0));
    if *count >= limit {
        return Err((60 - now.duration_since(*start).as_secs()).max(1));
    }
    *count += 1;
    Ok(())
}

async fn bounded_request(
    request: axum::extract::Request,
    next: Next,
    limits: &config::Limits,
) -> Response {
    match tokio::time::timeout(Duration::from_secs(limits.request_timeout_seconds), async {
        let (parts, body) = request.into_parts();
        let is_archive = parts.uri.path().starts_with("/api/v1/archive/");
        let max_bytes = if is_archive {
            archive::MAX_BODY
        } else {
            limits.max_request_bytes
        };
        match axum::body::to_bytes(body, max_bytes).await {
            Ok(bytes)
                if bytes.is_empty()
                    || is_archive
                    || matches!(parts.uri.path(), "/api/v1/updates/check" | "/api/v1/updates/install")
                    || parts.uri.path().starts_with("/api/v1/engine-registrations") =>
            {
                next.run(axum::extract::Request::from_parts(
                    parts,
                    axum::body::Body::from(bytes),
                ))
                .await
            }
            Ok(_) => failure(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "This endpoint does not accept a request body",
            ),
            Err(_) => failure(
                StatusCode::PAYLOAD_TOO_LARGE,
                "request_too_large",
                "Request body exceeds the limit or could not be read",
            ),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure(
            StatusCode::REQUEST_TIMEOUT,
            "request_timeout",
            "Request deadline exceeded",
        ),
    }
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    let supplied = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    // Compare all credential bytes, without an early mismatch return.
    let mut difference = supplied.len() ^ token.len();
    for (i, byte) in token.bytes().enumerate() {
        difference |= usize::from(byte ^ supplied.as_bytes().get(i).copied().unwrap_or(0));
    }
    difference == 0
}

async fn capabilities(State(shared): State<Shared>) -> Json<Capabilities> {
    let engines = shared.registry.engines.lock().unwrap().clone();
    Json(Capabilities {
        product: "Ironwood Companion".into(),
        server_version: env!("CARGO_PKG_VERSION").into(),
        protocol_version: PROTOCOL_VERSION.into(),
        services: Services {
            uci: !engines.is_empty(),
            archive: shared.archive.is_some(),
            explorer: shared.archive.is_some(),
            tablebases: !shared.config.tablebase_directories.is_empty(),
        },
        engine_ids: engines.iter().map(|e| e.id.clone()).collect(),
        archive_schema_version: shared.archive.as_ref().map(|_| archive::SCHEMA),
        tablebase_coverage: shared.tablebases.coverage_names().await,
    })
}

async fn engine_list(State(shared): State<Shared>) -> Json<Vec<EngineSummary>> {
    Json(
        shared
            .registry
            .engines
            .lock()
            .unwrap()
            .iter()
            .map(|e| EngineSummary {
                id: e.id.clone(),
                label: e.label.clone(),
            })
            .collect(),
    )
}

type TicketError = (StatusCode, &'static str, &'static str);

fn issue_ticket(shared: &Shared, origin: String, now: Instant) -> Result<String, TicketError> {
    let mut tickets = shared.tickets.lock().unwrap();
    tickets.retain(|_, value| value.expires > now);
    if tickets.len() >= shared.config.limits.max_pending_tickets {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            "Too many pending connections",
        ));
    }
    let mut bytes = [0u8; 32];
    if getrandom::fill(&mut bytes).is_err() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "random_unavailable",
            "Cannot create a connection ticket",
        ));
    }
    let value: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    tickets.insert(
        value.clone(),
        PendingTicket {
            origin,
            expires: now + Duration::from_secs(30),
        },
    );
    Ok(value)
}

fn consume_ticket(shared: &Shared, ticket: &str, origin: &str, now: Instant) -> bool {
    let mut tickets = shared.tickets.lock().unwrap();
    if !tickets
        .get(ticket)
        .is_some_and(|t| t.origin == origin && t.expires > now)
    {
        return false;
    }
    tickets.remove(ticket);
    true
}

async fn ticket(State(shared): State<Shared>, headers: HeaderMap) -> Response {
    let origin = headers["origin"].to_str().unwrap().to_owned();
    match issue_ticket(&shared, origin, Instant::now()) {
        Ok(ticket) => Json(Ticket {
            ticket,
            expires_in_seconds: 30,
        })
        .into_response(),
        Err((status, code, message)) => failure(status, code, message),
    }
}

async fn socket(
    State(shared): State<Shared>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let Ok(permit) = shared.sockets.clone().try_acquire_owned() else {
        return failure(
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            "Connection limit reached",
        );
    };
    let origin = headers["origin"].to_str().unwrap().to_owned();
    ws.max_message_size(shared.config.limits.max_request_bytes)
        .max_frame_size(shared.config.limits.max_request_bytes)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            session(socket, shared, origin).await;
        })
}

async fn send(socket: &mut WebSocket, value: ServerMessage) -> bool {
    matches!(
        tokio::time::timeout(
            Duration::from_secs(1),
            socket.send(Message::Text(serde_json::to_string(&value).unwrap().into()))
        )
        .await,
        Ok(Ok(()))
    )
}

async fn session(mut socket: WebSocket, shared: Shared, origin: String) {
    let first = tokio::select! {
        _ = shared.control.cancelled() => return,
        first = tokio::time::timeout(Duration::from_secs(shared.config.limits.authentication_timeout_seconds), socket.recv()) => first,
    };
    let valid = match first {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ClientMessage>(&text) {
            Ok(ClientMessage::Authenticate { ticket }) => {
                consume_ticket(&shared, &ticket, &origin, Instant::now())
            }
            _ => false,
        },
        _ => false,
    };
    if !valid {
        send(
            &mut socket,
            ServerMessage::Error {
                code: "unauthorized".into(),
                message: "Valid connection ticket required".into(),
            },
        )
        .await;
        let _ = socket.send(Message::Close(None)).await;
        return;
    }
    if !send(
        &mut socket,
        ServerMessage::Ready {
            protocol_version: PROTOCOL_VERSION.into(),
        },
    )
    .await
    {
        return;
    }
    engine_session(&mut socket, shared).await;
}

async fn engine_session(socket: &mut WebSocket, shared: Shared) {
    use engine::Engine;
    let mut engine: Option<Engine> = None;
    let mut tick = tokio::time::interval(Duration::from_millis(50));
    let mut last_input = Instant::now();
    loop {
        enum Event {
            Stop,
            Tick,
            Message(Option<Result<Message, axum::Error>>),
            Line(Option<engine::Result<String>>),
        }
        let event = tokio::select! {
            _ = shared.control.cancelled() => Event::Stop,
            _ = tick.tick() => Event::Tick,
            message = socket.recv() => Event::Message(message),
            line = async { match &mut engine { Some(engine) => engine.output.recv().await, None => std::future::pending().await } } => Event::Line(line),
        };
        let mut fatal = false;
        let mut error = None;
        match event {
            Event::Stop => break,
            Event::Tick => {
                if last_input.elapsed()
                    > Duration::from_secs(shared.config.limits.idle_timeout_seconds)
                {
                    break;
                }
                if engine
                    .as_ref()
                    .and_then(|e| e.active.as_ref())
                    .is_some_and(|a| a.deadline <= Instant::now())
                {
                    error = Some("Engine search or cancellation deadline exceeded");
                    fatal = true;
                }
            }
            Event::Line(Some(Ok(line))) => {
                if let Some(engine) = &mut engine {
                    match engine.output_message(&line) {
                        Ok(Some(message)) => {
                            if !send(socket, message).await {
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(message) => {
                            error = Some(message);
                            fatal = true;
                        }
                    }
                }
            }
            Event::Line(_) => {
                error = Some("Engine process exited or output failed");
                fatal = true;
            }
            Event::Message(Some(Ok(Message::Text(text)))) => {
                last_input = Instant::now();
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Ping { request_id }) if engine::valid_id(&request_id) => {
                        if !send(socket, ServerMessage::Pong { request_id }).await {
                            break;
                        }
                    }
                    Ok(ClientMessage::OpenEngine {
                        request_id,
                        engine_id,
                    }) if engine::valid_id(&request_id) => {
                        if engine.is_some() {
                            error = Some("Close the current engine before opening another");
                        } else if let Some(config) = {
                            shared
                                .registry
                                .engines
                                .lock()
                                .unwrap()
                                .iter()
                                .find(|e| e.id == engine_id)
                                .cloned()
                        } {
                            if let Ok(permit) = shared.engines.clone().try_acquire_owned() {
                                match Engine::spawn(
                                    &config,
                                    shared.config.engine_limits.clone(),
                                    permit,
                                    &shared.config.tablebase_directories,
                                ) {
                                    Ok(value) => {
                                        engine = Some(value);
                                        let result = tokio::select! { _ = shared.control.cancelled() => Err("Server is stopping"), result = engine.as_mut().unwrap().handshake() => result };
                                        match result {
                                            Ok(()) => {
                                                let e = engine.as_ref().unwrap();
                                                if !send(
                                                    socket,
                                                    ServerMessage::EngineOpened {
                                                        request_id,
                                                        session_id: e.session_id.clone(),
                                                        engine: e.details.clone(),
                                                    },
                                                )
                                                .await
                                                {
                                                    break;
                                                }
                                            }
                                            Err(message) => {
                                                error = Some(message);
                                                fatal = true;
                                            }
                                        }
                                    }
                                    Err(message) => error = Some(message),
                                }
                            } else {
                                error = Some("Server engine process limit reached");
                            }
                        } else {
                            error = Some("Unknown engine ID");
                        }
                    }
                    Ok(ClientMessage::Search {
                        session_id,
                        search_id,
                        position_id,
                        fen,
                        moves,
                        chess960,
                        new_game,
                        options,
                        limit,
                    }) => {
                        if let Some(e) = &mut engine {
                            if e.session_id != session_id {
                                error = Some("Unknown engine session");
                            } else {
                                // Validate first without losing the process on ordinary invalid input.
                                let was_busy = e.active.is_some();
                                let request = engine::SearchRequest {
                                    search_id: search_id.clone(),
                                    position_id: position_id.clone(),
                                    fen,
                                    moves,
                                    chess960,
                                    new_game,
                                    options,
                                    limit,
                                };
                                let result = tokio::select! { _ = shared.control.cancelled() => Err("Server is stopping"), result = e.search(request) => result };
                                match result {
                                    Ok(()) => {
                                        if !send(
                                            socket,
                                            ServerMessage::SearchStarted {
                                                session_id,
                                                search_id,
                                                position_id,
                                            },
                                        )
                                        .await
                                        {
                                            break;
                                        }
                                    }
                                    Err(message) => {
                                        error = Some(message);
                                        if (!was_busy && e.active.is_some())
                                            || message.starts_with("Engine input")
                                            || message.contains("readiness timed out")
                                            || message.starts_with("Engine exited")
                                        {
                                            fatal = true;
                                        }
                                    }
                                }
                            }
                        } else {
                            error = Some("Open an engine first");
                        }
                    }
                    Ok(ClientMessage::SetOption {
                        session_id,
                        request_id,
                        name,
                        value,
                    }) => {
                        if !engine::valid_id(&request_id) {
                            error = Some("Invalid option request ID");
                        } else if let Some(e) = &mut engine {
                            if e.session_id != session_id {
                                error = Some("Unknown engine session");
                            } else {
                                match e.set_idle_option(&name, &value).await {
                                    Ok(()) => {
                                        if !send(
                                            socket,
                                            ServerMessage::OptionSet {
                                                session_id,
                                                request_id,
                                            },
                                        )
                                        .await
                                        {
                                            break;
                                        }
                                    }
                                    Err(message) => error = Some(message),
                                }
                            }
                        } else {
                            error = Some("Open an engine first");
                        }
                    }
                    Ok(ClientMessage::Cancel {
                        session_id,
                        search_id,
                    }) => {
                        if let Some(e) = &mut engine {
                            if e.session_id != session_id {
                                error = Some("Unknown engine session");
                            } else {
                                match e.cancel(&search_id).await {
                                    Ok(false) => {
                                        if !send(
                                            socket,
                                            ServerMessage::Cancelled {
                                                session_id,
                                                search_id,
                                            },
                                        )
                                        .await
                                        {
                                            break;
                                        }
                                    }
                                    Ok(true) => {}
                                    Err(message) => error = Some(message),
                                }
                            }
                        } else {
                            error = Some("Open an engine first");
                        }
                    }
                    Ok(ClientMessage::CloseEngine { session_id }) => {
                        if engine.as_ref().is_some_and(|e| e.session_id == session_id) {
                            if let Some(mut e) = engine.take() {
                                e.close().await;
                            }
                            if !send(socket, ServerMessage::EngineClosed { session_id }).await {
                                break;
                            }
                        } else {
                            error = Some("Unknown engine session");
                        }
                    }
                    _ => error = Some("Invalid request"),
                }
            }
            Event::Message(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {
                last_input = Instant::now()
            }
            Event::Message(_) => break,
        }
        if let Some(message) = error
            && !send(
                socket,
                ServerMessage::Error {
                    code: if fatal {
                        "engine_failed"
                    } else if message.contains("busy") || message.contains("process limit") {
                        "busy"
                    } else {
                        "invalid_request"
                    }
                    .into(),
                    message: message.into(),
                },
            )
            .await
        {
            break;
        }
        if fatal && let Some(mut e) = engine.take() {
            e.close().await;
        }
    }
    if let Some(mut e) = engine {
        e.close().await;
    }
    let _ = tokio::time::timeout(Duration::from_secs(1), socket.send(Message::Close(None))).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    fn config() -> Config {
        Config {
            listen: "127.0.0.1:8790".into(),
            allowed_origins: vec!["https://ironwoodchess.com".into()],
            token_file: "owner-token".into(),
            allow_non_loopback: false,
            token: "a".repeat(64),
            limits: config::Limits::default(),
            engines: vec![],
            engine_limits: config::EngineLimits::default(),
            archive: None,
            tablebase_directories: vec![],
        }
    }
    #[tokio::test]
    async fn authentication_origin_and_version() {
        for (origin, token, version, expected) in [
            ("https://ironwoodchess.com", "a".repeat(64), "1", 200),
            ("https://evil.example", "a".repeat(64), "1", 403),
            ("null", "a".repeat(64), "1", 403),
            ("https://ironwoodchess.com", "wrong".into(), "1", 401),
            ("https://ironwoodchess.com", "a".repeat(64), "2", 426),
        ] {
            let request = Request::builder()
                .uri("/api/v1/capabilities")
                .header("origin", origin)
                .header("authorization", format!("Bearer {token}"))
                .header(PROTOCOL_HEADER, version)
                .body(Body::empty())
                .unwrap();
            let response = app(config()).unwrap().oneshot(request).await.unwrap();
            assert_eq!(response.status().as_u16(), expected);
            if expected == 200 {
                let bytes = axum::body::to_bytes(response.into_body(), 4096)
                    .await
                    .unwrap();
                let caps: Capabilities = serde_json::from_slice(&bytes).unwrap();
                assert!(
                    !caps.services.uci
                        && !caps.services.archive
                        && !caps.services.explorer
                        && !caps.services.tablebases
                );
            }
        }
    }
    #[test]
    fn desktop_status_counts_follow_live_permits() {
        let (_,control)=app_with_control(config()).unwrap();
        assert_eq!(control.active_connections(),0);
        assert_eq!(control.active_engines(),0);
        let connection=control.sockets.try_acquire().unwrap();
        let engine=control.engines.try_acquire().unwrap();
        assert_eq!(control.active_connections(),1);
        assert_eq!(control.active_engines(),1);
        drop(connection);drop(engine);
        assert_eq!(control.active_connections(),0);
        assert_eq!(control.active_engines(),0);
        control.shutdown();assert!(control.stopping());
    }
    #[test]
    fn updates_reserve_idle_engines_and_validation_atomically() {
        let (_,control)=app_with_control(config()).unwrap();
        let engine=control.engines.try_acquire().unwrap();
        assert!(control.reserve_update().is_none()); drop(engine);
        let validation=control.registrations.try_acquire().unwrap();
        assert!(control.reserve_update().is_none()); drop(validation);
        let reservation=control.reserve_update().unwrap();
        assert!(control.engines.try_acquire().is_err());
        assert!(control.registrations.try_acquire().is_err());
        drop(reservation);
        assert!(control.engines.try_acquire().is_ok());
        control.shutdown(); assert!(control.reserve_update().is_none());
    }
    #[test]
    fn tickets_expire_are_single_use_and_origin_bound() {
        let sockets = Arc::new(tokio::sync::Semaphore::new(16));
        let shared = Shared {
            registry: registration::Registry::new(vec![], None),
            tablebases: tablebases::Service::new(vec![]),
            archive: None,
            config: config(),
            tickets: Arc::default(),
            sockets: sockets.clone(),
            control: Control {
                updates: updates::Manager::default(),
                engines: Arc::new(tokio::sync::Semaphore::new(4)),
                max_engines: 4,
                registrations: Arc::new(tokio::sync::Semaphore::new(1)),
                tablebases: tablebases::Service::new(vec![]),
                background: Arc::new(tokio::sync::Semaphore::new(1)),
                stop: tokio::sync::watch::channel(false).0,
                sockets,
                max_connections: 16,
            },
            rate: Arc::default(),
            engines: Arc::new(tokio::sync::Semaphore::new(4)),
        };
        let now = Instant::now();
        let ticket = issue_ticket(&shared, "https://ironwoodchess.com".into(), now).unwrap();
        assert!(!consume_ticket(
            &shared,
            &ticket,
            "https://evil.example",
            now
        ));
        assert!(consume_ticket(
            &shared,
            &ticket,
            "https://ironwoodchess.com",
            now
        ));
        assert!(!consume_ticket(
            &shared,
            &ticket,
            "https://ironwoodchess.com",
            now
        ));
        let ticket = issue_ticket(&shared, "https://ironwoodchess.com".into(), now).unwrap();
        assert!(!consume_ticket(
            &shared,
            &ticket,
            "https://ironwoodchess.com",
            now + Duration::from_secs(31)
        ));
    }
    #[test]
    fn configuration_fails_closed() {
        let mut value = config();
        value.listen = "0.0.0.0:8790".into();
        assert!(value.validate().is_err());
        value.allow_non_loopback = true;
        assert!(value.validate().is_ok());
        value.allowed_origins = vec!["*".into()];
        assert!(value.validate().is_err());
    }

    #[tokio::test]
    async fn probes_shutdown_limits_and_structured_errors() {
        let mut config = config();
        config.limits.max_request_bytes = 8;
        let (router, control) = app_with_control(config).unwrap();
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let request = |method: &str, body: &'static str| {
            Request::builder()
                .uri("/api/v1/tickets")
                .method(method)
                .header("origin", "https://ironwoodchess.com")
                .header("authorization", format!("Bearer {}", "a".repeat(64)))
                .header(PROTOCOL_HEADER, "1")
                .body(Body::from(body))
                .unwrap()
        };
        let response = router
            .clone()
            .oneshot(request("POST", "123456789"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let response = router.clone().oneshot(request("DELETE", "")).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let error: ApiError = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(error.code, "method_not_allowed");
        control.shutdown();
        assert_eq!(
            router
                .oneshot(
                    Request::builder()
                        .uri("/readyz")
                        .body(Body::empty())
                        .unwrap()
                )
                .await
                .unwrap()
                .status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn request_budget_keeps_mutations_and_backups_in_operations() {
        for path in ["/api/v1/archive/import", "/api/v1/archive/restore",
            "/api/v1/archive/games/id/delete", "/api/v1/archive/explorer/rebuild",
            "/api/v1/tickets", "/api/v1/engine-registrations"] {
            assert_eq!(request_budget(&Method::POST, path), RequestBudget::Operations);
        }
        assert_eq!(request_budget(&Method::GET, "/api/v1/archive/backup"), RequestBudget::Operations);
        assert_eq!(request_budget(&Method::POST, "/api/v1/archive/export"), RequestBudget::ArchiveReads);
        assert_eq!(request_budget(&Method::GET, "/api/v1/archive/games/id"), RequestBudget::ArchiveReads);
        assert_eq!(request_budget(&Method::OPTIONS, "/api/v1/tickets"), RequestBudget::Preflight);
        let limits: config::Limits = serde_json::from_str(r#"{"requests_per_minute":17}"#).unwrap();
        assert_eq!(limits.requests_per_minute, 17);
        assert_eq!(limits.archive_reads_per_minute, 1200);
        assert_eq!(limits.preflight_requests_per_minute, 2400);
    }

    #[tokio::test]
    async fn request_budgets_are_independent_and_keep_authentication() {
        let mut config = config();
        config.limits.requests_per_minute = 1;
        config.limits.archive_reads_per_minute = 2;
        config.limits.preflight_requests_per_minute = 1;
        let router = app(config).unwrap();
        // Disabled archive rejects the export after authentication; it still spends a read token.
        for (method, path, credential, expected) in [
            (Method::GET, "/api/v1/capabilities", true, 200),
            (Method::GET, "/api/v1/capabilities", true, 429),
            (Method::OPTIONS, "/api/v1/archive/games", false, 204),
            (Method::OPTIONS, "/api/v1/archive/games", false, 429),
            (Method::GET, "/api/v1/archive/games", false, 401),
            (Method::POST, "/api/v1/archive/export", true, 400),
            (Method::GET, "/api/v1/archive/stats", true, 429),
        ] {
            let mut request = Request::builder().method(method.clone()).uri(path)
                .header("origin", "https://ironwoodchess.com")
                .header(PROTOCOL_HEADER, "1")
                .header("access-control-request-method", "GET")
                .header("content-type", "application/json");
            if credential {
                request = request.header("authorization", format!("Bearer {}", "a".repeat(64)));
            }
            let response = router.clone().oneshot(request.body(if method == Method::POST { Body::from(r#"{"ids":["test"],"format":"json"}"#) } else { Body::empty() }).unwrap()).await.unwrap();
            assert_eq!(response.status().as_u16(), expected, "{path}");
            if expected == 429 {
                assert_eq!(response.headers()["access-control-expose-headers"], "Retry-After");
                let retry: u64 = response.headers()["retry-after"].to_str().unwrap().parse().unwrap();
                assert!((1..=60).contains(&retry));
            }
        }
    }

    #[tokio::test]
    async fn rate_limit_returns_cors_error_and_preserves_health() {
        let mut config = config();
        config.limits.requests_per_minute = 1;
        let router = app(config).unwrap();
        for expected in [200, 429] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/capabilities")
                        .header("origin", "https://ironwoodchess.com")
                        .header("authorization", format!("Bearer {}", "a".repeat(64)))
                        .header(PROTOCOL_HEADER, "1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status().as_u16(), expected);
            assert_eq!(
                response.headers()["access-control-allow-origin"],
                "https://ironwoodchess.com"
            );
        }
        assert_eq!(
            router
                .oneshot(
                    Request::builder()
                        .uri("/healthz")
                        .body(Body::empty())
                        .unwrap()
                )
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
}
