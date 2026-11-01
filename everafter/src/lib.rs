use axum::{extract::{Path, State, DefaultBodyLimit}, http::StatusCode, response::Html, routing::{get, post}, Json, Router};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
mod invitations;
pub mod checklists;
mod auth;
mod organization;

type Db = Arc<Mutex<Connection>>;
type ApiResult = Result<Json<Value>, (StatusCode, String)>;
fn error(e: impl std::fmt::Display) -> (StatusCode, String) { (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()) }

async fn records(State(db): State<Db>) -> ApiResult {
    let db = db.lock().map_err(error)?;
    let mut stmt = db.prepare("SELECT kind, data FROM records ORDER BY rowid").map_err(error)?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(error)?;
    let mut result = json!({"guests":[],"tasks":[],"vendors":[],"rides":[],"settings":[],"categories":[],"tiers":[]});
    for row in rows { let (kind, data) = row.map_err(error)?; if let Some(list) = result.get_mut(&kind).and_then(Value::as_array_mut) { list.push(serde_json::from_str(&data).map_err(error)?); } }
    Ok(Json(result))
}
fn valid_kind(k: &str) -> bool { ["guests","tasks","vendors","rides","settings","categories","tiers"].contains(&k) }
async fn save(State(db): State<Db>, Path(kind): Path<String>, Json(mut data): Json<Value>) -> ApiResult {
    if !valid_kind(&kind) || !data.is_object() { return Err((StatusCode::BAD_REQUEST,"Invalid record".into())); }
    let id = data.get("id").and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    data["id"] = json!(id);
    if kind == "guests" && data.get("token").is_none() { data["token"] = json!(uuid::Uuid::new_v4().to_string()); }
    let mut db = db.lock().map_err(error)?;
    if kind == "settings" {
        let selected=data.get("weddingKind").and_then(Value::as_str).unwrap_or("Choose later");
        if !checklists::KINDS.contains(&selected) {return Err((StatusCode::BAD_REQUEST,"Choose a supported wedding style.".into()));}
    }
    let tx=db.transaction().map_err(error)?;
    organization::validate_and_rename(&tx,&kind,&id,&mut data).map_err(|m|(StatusCode::BAD_REQUEST,m))?;
    let old_style:Option<String>=if kind=="settings" {tx.query_row("SELECT json_extract(data,'$.weddingKind') FROM records WHERE kind='settings' AND id=?1",[&id],|r|r.get::<_,Option<String>>(0)).ok().flatten()}else{None};
    // Preserve the unguessable guest link on subsequent planner edits.
    if kind == "guests" { if let Ok(old) = tx.query_row("SELECT data FROM records WHERE kind=?1 AND id=?2", params![kind,id], |r| r.get::<_,String>(0)) { let old: Value = serde_json::from_str(&old).map_err(error)?; data["token"] = old["token"].clone(); } }
    tx.execute("INSERT INTO records(kind,id,data) VALUES(?1,?2,?3) ON CONFLICT(kind,id) DO UPDATE SET data=excluded.data",params![kind,id,data.to_string()]).map_err(error)?;
    if kind=="settings" {
        let selected=data.get("weddingKind").and_then(Value::as_str).unwrap_or("Choose later");
        if old_style.as_deref()!=Some(selected) { checklists::apply(&tx,selected,data.get("date").and_then(Value::as_str).unwrap_or("")).map_err(error)?; }
    }
    tx.commit().map_err(error)?;
    Ok(Json(data))
}
async fn remove(State(db): State<Db>, Path((kind,id)): Path<(String,String)>) -> ApiResult {
    let db=db.lock().map_err(error)?;
    organization::check_delete(&db,&kind,&id).map_err(|m|(StatusCode::CONFLICT,m))?;
    db.execute("DELETE FROM records WHERE kind=?1 AND id=?2", params![kind,id]).map_err(error)?;
    Ok(Json(json!({"ok":true})))
}
async fn guest_get(State(db): State<Db>, Path(token): Path<String>) -> ApiResult {
    let db = db.lock().map_err(error)?;
    let raw: String = db.query_row("SELECT data FROM records WHERE kind='guests' AND json_extract(data,'$.token')=?1",[token],|r|r.get(0)).map_err(|_|(StatusCode::NOT_FOUND,"Guest link not found".into()))?;
    let g: Value = serde_json::from_str(&raw).map_err(error)?;
    // Guest responses never expose private tiers, notes, budgets, or other guests.
    let mut public = json!({});
    for key in ["name","rsvp","meal","address","needsHotel","needsRide","arrival","passengers","hotel","room","pickup","driver","phone"] { public[key] = g.get(key).cloned().unwrap_or(Value::Null); }
    Ok(Json(public))
}
async fn guest_update(State(db): State<Db>, Path(token): Path<String>, Json(input): Json<Value>) -> ApiResult {
    let db = db.lock().map_err(error)?;
    let raw: String = db.query_row("SELECT data FROM records WHERE kind='guests' AND json_extract(data,'$.token')=?1",[token],|r|r.get(0)).map_err(|_|(StatusCode::NOT_FOUND,"Guest link not found".into()))?;
    let mut g: Value = serde_json::from_str(&raw).map_err(error)?;
    for key in ["rsvp","meal","address","needsHotel","needsRide","arrival","passengers"] { if let Some(v) = input.get(key) { g[key] = v.clone(); } }
    db.execute("UPDATE records SET data=?1 WHERE kind='guests' AND id=?2",params![g.to_string(),g["id"].as_str()]).map_err(error)?;
    Ok(Json(json!({"ok":true})))
}
pub type AppRouter = Router;
pub fn router(database_path: &std::path::Path) -> AppRouter {
    let db = Connection::open(database_path).expect("Open app database");
    db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS records(kind TEXT NOT NULL,id TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(kind,id));").expect("Initialize database");
    invitations::initialize(&db).expect("Initialize invitation studio");
    organization::initialize(&db).expect("Initialize guest categories and tiers");
    let app = Router::new().route("/",get(|| async { Html(include_str!("../static/index.html")) }))
        .route("/guest/{token}",get(|| async { Html(include_str!("../static/index.html")) }))
        .route("/api/records",get(records)).route("/api/records/{kind}",post(save))
        .route("/api/guests/organize",post(organization::bulk))
        .route("/api/guests/import",post(organization::import_contacts))
        .route("/api/records/{kind}/{id}",axum::routing::delete(remove))
        .route("/api/guest/{token}",get(guest_get).post(guest_update))
        .route("/invitation.js",get(|| async { ([(axum::http::header::CONTENT_TYPE,"text/javascript; charset=utf-8")], include_str!("../static/invitation.js")) }))
        .route("/organization.js",get(|| async { ([(axum::http::header::CONTENT_TYPE,"text/javascript; charset=utf-8")], include_str!("../static/organization.js")) }))
        .route("/contacts.js",get(|| async { ([(axum::http::header::CONTENT_TYPE,"text/javascript; charset=utf-8")], include_str!("../static/contacts.js")) }))
        .route("/mobile.js",get(|| async { ([(axum::http::header::CONTENT_TYPE,"text/javascript; charset=utf-8")], include_str!("../static/mobile.js")) }))
        .route("/mobile.css",get(|| async { ([(axum::http::header::CONTENT_TYPE,"text/css; charset=utf-8")], include_str!("../static/mobile.css")) }))
        .route("/api/ai/status",get(invitations::status))
        .route("/api/ai/connect",post(invitations::connect))
        .route("/api/invitations",get(invitations::list))
        .route("/api/invitations/generate",post(invitations::generate))
        .route("/api/invitations/{id}/image",get(invitations::image))
        .route("/api/invitations/{id}",axum::routing::delete(invitations::delete))
        .layer(DefaultBodyLimit::max(1024*1024))
        .layer(axum::middleware::from_fn_with_state(Arc::new(auth::Config::from_env()),auth::protect))
        .with_state(Arc::new(Mutex::new(db)));
    app
}

pub async fn serve(listener:tokio::net::TcpListener,router:AppRouter)->std::io::Result<()> { axum::serve(listener,router).await }

pub fn copy_database_once(source:&std::path::Path,destination:&std::path::Path)->rusqlite::Result<()> {
    if !destination.exists() { let source=Connection::open(source)?;source.backup(rusqlite::DatabaseName::Main,destination,None)?; }
    Ok(())
}
