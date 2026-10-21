use super::{Shared, failure};
use crate::archive::{self, Backup, Document, Filter, Result};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/v1/archive/games", get(list))
        .route("/api/v1/archive/library", get(library))
        .route("/api/v1/archive/stats", get(stats))
        .route("/api/v1/archive/games/{id}", get(detail))
        .route("/api/v1/archive/games/{id}/analysis-runs", get(run_list))
        .route("/api/v1/archive/games/{id}/analysis-runs/delete", post(run_delete))
        .route("/api/v1/archive/games/{id}/analysis-runs/{run}", get(run_detail).post(run_save))
        .route("/api/v1/archive/games/{id}/update", post(update))
        .route("/api/v1/archive/games/{id}/delete", post(delete))
        .route("/api/v1/archive/import", post(import))
        .route("/api/v1/archive/export", post(export))
        .route(
            "/api/v1/archive/collections",
            get(collections).post(create_collection),
        )
        .route("/api/v1/archive/collections/{id}", post(edit_collection))
        .route("/api/v1/archive/backup", get(backup))
        .route("/api/v1/archive/restore", post(restore))
        .route("/api/v1/archive/explorer", get(explorer_query))
        .route("/api/v1/archive/explorer/status", get(explorer_status))
        .route("/api/v1/archive/explorer/rebuild", post(explorer_rebuild))
}
async fn run_list(State(shared):State<Shared>,Path(id):Path<String>)->Response {
 response(async{store(&shared)?.run(move|db|crate::analysis_runs::list(db,&id)).await.and_then(bounded)}.await)
}
async fn run_delete(State(shared):State<Shared>,Path(id):Path<String>,Json(request):Json<crate::analysis_runs::Delete>)->Response {
 response(async{store(&shared)?.run(move|db|crate::analysis_runs::delete(db,&id,request)).await}.await)
}
async fn run_detail(State(shared):State<Shared>,Path((id,run)):Path<(String,String)>)->Response {
 response(async{store(&shared)?.run(move|db|crate::analysis_runs::get(db,&id,&run)).await}.await)
}
async fn run_save(State(shared):State<Shared>,Path((id,run)):Path<(String,String)>,Json(request):Json<crate::analysis_runs::Save>)->Response {
 response(async{store(&shared)?.run(move|db|crate::analysis_runs::save(db,&id,&run,request)).await}.await)
}
async fn explorer_query(
    State(shared): State<Shared>,
    Query(query): Query<crate::explorer::Query>,
) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| crate::explorer::query(db, query))
                .await
        }
        .await,
    )
}
async fn explorer_status(State(shared): State<Shared>) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| crate::explorer::status(db))
                .await
        }
        .await,
    )
}
async fn explorer_rebuild(State(shared): State<Shared>) -> Response {
    response(async { store(&shared)?.run(crate::explorer::rebuild).await }.await)
}
fn response<T: serde::Serialize>(result: Result<T>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(archive::Error(code, message)) => failure(
            match code {
                "not_found" => StatusCode::NOT_FOUND,
                "conflict" => StatusCode::CONFLICT,
                "busy" => StatusCode::TOO_MANY_REQUESTS,
                "archive_failed" => StatusCode::INTERNAL_SERVER_ERROR,
                _ => StatusCode::BAD_REQUEST,
            },
            code,
            message,
        ),
    }
}
fn store(shared: &Shared) -> Result<archive::Archive> {
    shared.archive.clone().ok_or(archive::Error(
        "archive_disabled",
        "Archive is disabled in local configuration",
    ))
}
async fn library(State(shared): State<Shared>, Query(filter): Query<crate::archive_library::Filter>) -> Response {
    response(async { store(&shared)?.run(move |db| crate::archive_library::query(db, filter)).await.and_then(bounded) }.await)
}
async fn stats(State(shared): State<Shared>) -> Response {
    response(async { store(&shared)?.run(|db| archive::stats(db)).await }.await)
}
async fn list(State(shared): State<Shared>, Query(filter): Query<Filter>) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| archive::list(db, filter))
                .await
        }
        .await,
    )
}
async fn detail(State(shared): State<Shared>, Path(id): Path<String>) -> Response {
    response(async { store(&shared)?.run(move |db| archive::get(db, &id)).await }.await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    documents: Option<Vec<Document>>,
    pgn: Option<String>,
    collection: Option<String>,
}
async fn import(State(shared): State<Shared>, Json(request): Json<Import>) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| {
                    let documents = match (request.documents, request.pgn) {
                        (Some(d), None) => d,
                        (None, Some(p)) => crate::archive_pgn::documents(&p)?,
                        _ => return Err(archive::invalid("Provide documents or PGN, exclusively")),
                    };
                    archive::import(db, documents, request.collection)
                })
                .await
        }
        .await,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    revision: u64,
    document: Document,
}
async fn update(
    State(shared): State<Shared>,
    Path(id): Path<String>,
    Json(request): Json<Update>,
) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| archive::update(db, &id, request.revision, request.document))
                .await
        }
        .await,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    revision: u64,
    #[serde(default)] analysis_runs: Option<Vec<(String,u64)>>,
}
async fn delete(
    State(shared): State<Shared>,
    Path(id): Path<String>,
    Json(request): Json<Revision>,
) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| archive::delete_with_runs(db, &id, request.revision,request.analysis_runs))
                .await
        }
        .await,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectionEdit {
    #[serde(default)]
    revision: u64,
    action: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    games: Vec<String>,
}
async fn collections(State(shared): State<Shared>) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| archive::collections(db))
                .await
        }
        .await,
    )
}
async fn create_collection(
    State(shared): State<Shared>,
    Json(request): Json<CollectionEdit>,
) -> Response {
    collection_request(shared, None, request).await
}
async fn edit_collection(
    State(shared): State<Shared>,
    Path(id): Path<String>,
    Json(request): Json<CollectionEdit>,
) -> Response {
    collection_request(shared, Some(id), request).await
}
async fn collection_request(
    shared: Shared,
    id: Option<String>,
    request: CollectionEdit,
) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| {
                    archive::collection_edit(
                        db,
                        id.as_deref(),
                        request.revision,
                        &request.action,
                        &request.name,
                        &request.games,
                    )
                })
                .await
        }
        .await,
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    ids: Vec<String>,
    format: String,
}
async fn export(State(shared): State<Shared>, Json(request): Json<Export>) -> Response {
    response(async{store(&shared)?.run(move|db|{
    if request.ids.is_empty() || request.ids.len()>archive::MAX_BATCH{return Err(archive::invalid("Select 1 to 1000 games"));}
    let tx=db.transaction()?;let documents=request.ids.iter().map(|id|archive::get(&tx,id).and_then(|mut g|{if request.format=="json"{g.document.analysis_runs=crate::analysis_runs::all(&tx,id)?;}Ok(g.document)})).collect::<Result<Vec<_>>>()?;tx.commit()?;
    let value=match request.format.as_str(){"json"=>json!({"format":"ironwood-companion-transfer","version":1,"documents":documents}),"pgn"=>json!({"pgn":documents.iter().map(|d|d.pgn.as_str()).collect::<Vec<_>>().join("\n\n")}),_=>return Err(archive::invalid("Unsupported export format"))};
    bounded(value)
}).await}.await)
}
fn bounded(value: Value) -> Result<Value> {
    if serde_json::to_vec(&value)?.len() > archive::MAX_BODY {
        return Err(archive::invalid(
            "Export exceeds 16 MiB; select a smaller batch or use native SQLite backup",
        ));
    }
    Ok(value)
}
async fn backup(State(shared): State<Shared>) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| bounded(serde_json::to_value(archive::backup(db)?)?))
                .await
        }
        .await,
    )
}
async fn restore(State(shared): State<Shared>, Json(backup): Json<Backup>) -> Response {
    response(
        async {
            store(&shared)?
                .run(move |db| archive::restore(db, backup))
                .await
        }
        .await,
    )
}
