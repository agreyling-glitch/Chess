use ironwood_server::{
    Config, app_with_registration,
    config::{DataDirectory, Error},
    log_event,
};
#[cfg(windows)]
mod tray;
use std::{
    future::IntoFuture,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    time::Duration,
};

const HELP: &str = "Ironwood Companion\n\
Usage: ironwood-server [serve|init|rotate-token] [--data-dir <directory>] [--docker]\n\
       ironwood-server archive-backup --data-dir <directory> --output <new.sqlite3>\n\
       ironwood-server archive-restore --data-dir <directory> --input <backup.sqlite3>\n\
       ironwood-server --config <config.json>\n\
       ironwood-server healthcheck [--address <IP:port>]\n\
First serve initializes an empty data directory automatically. init does not start\n\
the server. rotate-token requires the server to be stopped. Credentials are never\n\
printed. --docker binds inside the container; publish only to host loopback.\n\
Windows: ironwood-companion opens the tray; ironwood-server --tray also enables it.\n\
Use --no-tray for a headless server.\n";

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        if error
            .downcast_ref::<ironwood_server::config::DataDirectoryInUse>()
            .is_some()
            && std::env::args_os().any(|arg| arg == "--start-if-needed")
        {
            println!("Companion is already running.");
            #[cfg(windows)]
            if desktop_launch() {
                tray::notice("Companion is already running.");
            }
            std::process::exit(20); // Launcher translates this expected condition to success.
        }
        log_event(
            "startup_failed",
            serde_json::json!({"message":error.to_string()}),
        );
        #[cfg(windows)]
        if desktop_launch() {
            tray::notice(&format!("Companion could not start: {error}"));
        }
        std::process::exit(1);
    }
}

fn desktop_launch() -> bool {
    cfg!(windows)
        && std::env::current_exe()
            .ok()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .is_some_and(|s| s.eq_ignore_ascii_case("ironwood-companion"))
}

fn default_directory() -> PathBuf {
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(root).join("Ironwood").join("Companion");
    }
    PathBuf::from("companion-data")
}

async fn run() -> Result<(), Error> {
    let mut action = "serve".to_owned();
    let mut directory = default_directory();
    let mut config_file = None;
    let mut docker = false;
    let mut tray_enabled = desktop_launch();
    let mut start_if_needed = false;
    let mut update_pending: Option<PathBuf> = None;
    let mut transfer_path: Option<PathBuf> = None;
    let mut health_address = "127.0.0.1:8790".to_owned();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => {
                print!("{HELP}");
                return Ok(());
            }
            Some("--version") => {
                println!("Ironwood Companion {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Some(
                "serve" | "init" | "rotate-token" | "healthcheck" | "archive-backup"
                | "archive-restore",
            ) if action == "serve" => action = arg.to_string_lossy().into_owned(),
            Some("--data-dir") => directory = args.next().ok_or("Missing data directory")?.into(),
            Some("--config") => {
                config_file = Some(PathBuf::from(
                    args.next().ok_or("Missing configuration path")?,
                ))
            }
            Some("--address") => {
                health_address = args
                    .next()
                    .ok_or("Missing healthcheck address")?
                    .to_string_lossy()
                    .into_owned()
            }
            Some("--docker") => docker = true,
            Some("--tray") => tray_enabled = true,
            Some("--no-tray") => tray_enabled = false,
            Some("--start-if-needed") => start_if_needed = true,
            Some("--update-pending") if cfg!(windows) => {
                update_pending = Some(args.next().ok_or("Missing update marker")?.into());
            }
            Some("--output" | "--input") => {
                transfer_path = Some(args.next().ok_or("Missing snapshot path")?.into())
            }
            _ => return Err("Invalid arguments; use --help".into()),
        }
    }
    if start_if_needed && action != "serve" {
        return Err("--start-if-needed is only supported for serve".into());
    }
    if tray_enabled && (!cfg!(windows) || docker || action != "serve") {
        return Err("The desktop tray is supported only for native Windows serve; use --no-tray for headless operation".into());
    }
    if action == "healthcheck" {
        return healthcheck(&health_address);
    }
    if config_file.is_some() && (!matches!(action.as_str(), "serve" | "archive-backup" | "archive-restore") || docker) {
        return Err("--config is only supported for serve without --docker".into());
    }
    let (storage, path) = if let Some(path) = config_file {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."));
        (DataDirectory::open(parent)?, path)
    } else {
        let storage = DataDirectory::open(&directory)?;
        if action == "rotate-token" {
            storage.rotate()?;
            log_event(
                "credential_rotated",
                serde_json::json!({"restart_required":true}),
            );
            return Ok(());
        }
        let created = storage.initialize(docker)?;
        log_event(
            if created {
                "initialized"
            } else {
                "configuration_loaded"
            },
            serde_json::json!({"version":env!("CARGO_PKG_VERSION")}),
        );
        if action == "init" {
            return Ok(());
        }
        let path = storage.path.join("config.json");
        (storage, path)
    };
    storage.prepare_layout(&path)?;
    let config = Config::read(&path)?;
    if action == "archive-backup" || action == "archive-restore" {
        let archive = config.archive.as_deref().ok_or("Archive is disabled")?;
        let transfer = transfer_path.ok_or("Snapshot path required")?;
        if action == "archive-backup" {
            ironwood_server::archive::snapshot(archive, &transfer)?;
        } else {
            ironwood_server::archive::snapshot(&transfer, archive)?;
        }
        log_event(
            "archive_snapshot_complete",
            serde_json::json!({"action":action}),
        );
        return Ok(());
    }
    let address: SocketAddr = config.listen.parse()?;
    let deadline = Duration::from_secs(config.limits.shutdown_timeout_seconds);
    #[cfg(windows)]
    let tray_config = config.clone();
    let (router, control) = app_with_registration(config, Some(std::fs::canonicalize(&path)?))?;
    // An updated process must pass startup checks before any client can modify
    // its data. The helper removes this marker only after successful validation.
    let router = if let Some(marker) = update_pending {
        router.layer(axum::middleware::from_fn(move |request: axum::extract::Request, next: axum::middleware::Next| {
            let pending = marker.exists();
            async move {
                if pending && request.uri().path() != "/readyz" {
                    use axum::response::IntoResponse;
                    axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response()
                } else { next.run(request).await }
            }
        }))
    } else { router };
    let listener = tokio::net::TcpListener::bind(address).await?;
    control.updates.configure(ironwood_server::updates::Context {
        data: storage.path.clone(), config: path.clone(), address: listener.local_addr()?.to_string(),
        native_windows: cfg!(windows) && tray_enabled && !docker, container: docker,
    });
    #[cfg(windows)]
    let _tray = if tray_enabled {
        Some(tray::start(
            tray_config,
            storage.path.clone(),
            path.clone(),
            listener.local_addr()?,
            control.clone(),
            desktop_launch(),
        )?)
    } else {
        None
    };
    log_event(
        "listening",
        serde_json::json!({"address":listener.local_addr()?.to_string(),"version":env!("CARGO_PKG_VERSION")}),
    );
    let shutdown = control.clone();
    let server = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown.cancelled().await;
    })
    .into_future();
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => { result?; return Ok(()); },
        _ = shutdown_signal() => {},
        _ = control.cancelled() => {},
    }
    control.shutdown();
    log_event("shutdown_started", serde_json::json!({}));
    if tokio::time::timeout(deadline, async {
        server.await?;
        control.drained().await;
        Ok::<_, std::io::Error>(())
    })
    .await
    .is_err()
    {
        log_event(
            "shutdown_deadline",
            serde_json::json!({"seconds":deadline.as_secs()}),
        );
    }
    drop(storage);
    log_event("stopped", serde_json::json!({}));
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
    }
    #[cfg(windows)]
    {
        let mut close = tokio::signal::windows::ctrl_close().expect("Windows close handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = close.recv() => {} }
    }
}

fn healthcheck(address: &str) -> Result<(), Error> {
    let mut stream = TcpStream::connect_timeout(&address.parse()?, Duration::from_secs(2))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(b"GET /readyz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
    let mut bytes = [0u8; 13];
    stream.read_exact(&mut bytes)?;
    if &bytes != b"HTTP/1.1 200 " {
        return Err("Companion is not ready".into());
    }
    Ok(())
}
