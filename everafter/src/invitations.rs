use axum::{extract::{Path, State}, http::{header, StatusCode}, Json};
use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{sync::Mutex, time::Duration};
use crate::{Db, ApiResult, error};

// Never store credentials in SQLite or return them to the browser.
static SESSION_KEY: Mutex<Option<String>> = Mutex::new(None);
fn api_key() -> Option<String> {
    SESSION_KEY.lock().ok().and_then(|k| k.clone())
        .or_else(|| std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.trim().is_empty()))
}
pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS invitations(id TEXT PRIMARY KEY,prompt TEXT NOT NULL,details TEXT NOT NULL,status TEXT NOT NULL,image TEXT,error TEXT,created TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))); UPDATE invitations SET status='failed',error='The server restarted before this design finished. Please generate again.' WHERE status='generating';")
}
pub async fn status() -> Json<Value> {
    Json(json!({"connected":api_key().is_some(),"model":model()}))
}
fn model() -> String { std::env::var("OPENAI_IMAGE_MODEL").unwrap_or_else(|_|"gpt-image-1.5".into()) }
pub async fn connect(Json(body): Json<Value>) -> ApiResult {
    let key=body.get("key").and_then(Value::as_str).unwrap_or("").trim();
    if !key.is_empty() && (!key.starts_with("sk-") || key.len()<20 || key.len()>512 || key.contains(char::is_whitespace)) {
        return Err((StatusCode::BAD_REQUEST,"Enter a valid OpenAI API key, or leave blank to disconnect.".into()));
    }
    *SESSION_KEY.lock().map_err(error)?=if key.is_empty(){None}else{Some(key.to_string())};
    Ok(Json(json!({"connected":api_key().is_some()})))
}
pub async fn list(State(db): State<Db>) -> ApiResult {
    let db=db.lock().map_err(error)?;
    let mut stmt=db.prepare("SELECT id,prompt,details,status,error,created FROM invitations ORDER BY created DESC,rowid DESC").map_err(error)?;
    let rows=stmt.query_map([],|r| Ok(json!({"id":r.get::<_,String>(0)?,"prompt":r.get::<_,String>(1)?,"details":serde_json::from_str::<Value>(&r.get::<_,String>(2)?).unwrap_or(json!({})),"status":r.get::<_,String>(3)?,"error":r.get::<_,Option<String>>(4)?,"created":r.get::<_,String>(5)?}))).map_err(error)?;
    let values:Result<Vec<_>,_>=rows.collect();
    Ok(Json(json!(values.map_err(error)?)))
}
pub async fn image(State(db): State<Db>,Path(id):Path<String>) -> Result<([(header::HeaderName,&'static str);2],Vec<u8>),(StatusCode,String)> {
    let db=db.lock().map_err(error)?;
    let image:String=db.query_row("SELECT image FROM invitations WHERE id=?1 AND status='ready'",[id],|r|r.get(0)).map_err(|_|(StatusCode::NOT_FOUND,"Design not available".into()))?;
    let bytes=STANDARD.decode(image).map_err(error)?;
    Ok(([(header::CONTENT_TYPE,"image/png"),(header::CACHE_CONTROL,"private, max-age=86400")],bytes))
}
pub async fn delete(State(db):State<Db>,Path(id):Path<String>)->ApiResult {
    let db=db.lock().map_err(error)?;
    let affected=db.execute("DELETE FROM invitations WHERE id=?1 AND status!='generating'",[id]).map_err(error)?;
    if affected==0 {return Err((StatusCode::CONFLICT,"Design is still generating or was already removed.".into()));}
    Ok(Json(json!({"ok":true})))
}
pub async fn generate(State(db):State<Db>,Json(body):Json<Value>)->ApiResult {
    let prompt=body.get("prompt").and_then(Value::as_str).unwrap_or("").trim();
    if prompt.len()<12 || prompt.len()>4000 {return Err((StatusCode::BAD_REQUEST,"Describe your design in 12–4,000 characters.".into()));}
    let key=api_key().ok_or((StatusCode::PRECONDITION_REQUIRED,"Connect AI before generating an invitation.".into()))?;
    let details=body.get("details").filter(|d|d.is_object()).cloned().unwrap_or(json!({}));
    for field in ["names","date","venue","time","dress"] {
        if details.get(field).and_then(Value::as_str).unwrap_or("").len()>500 {return Err((StatusCode::BAD_REQUEST,"Wedding detail is too long.".into()));}
    }
    for field in ["names","date","venue"] {
        if details.get(field).and_then(Value::as_str).unwrap_or("").trim().is_empty() {return Err((StatusCode::BAD_REQUEST,"Add names, a date, and a venue before generating.".into()));}
    }
    let id=uuid::Uuid::new_v4().to_string();
    {
        let db=db.lock().map_err(error)?;
        let busy:i64=db.query_row("SELECT count(*) FROM invitations WHERE status='generating'",[],|r|r.get(0)).map_err(error)?;
        if busy>0 {return Err((StatusCode::CONFLICT,"A design is already generating. Please wait for it to finish.".into()));}
        db.execute("INSERT INTO invitations(id,prompt,details,status) VALUES(?1,?2,?3,'generating')",params![id,prompt,details.to_string()]).map_err(error)?;
    }
    let job_id=id.clone(); let user_prompt=prompt.to_owned();
    tokio::spawn(async move {
        let result=generate_image(&key,&user_prompt,&details).await;
        if let Ok(db)=db.lock() {
            match result {
                Ok(b64)=>{let _=db.execute("UPDATE invitations SET status='ready',image=?1 WHERE id=?2",params![b64,job_id]);},
                Err(message)=>{let _=db.execute("UPDATE invitations SET status='failed',error=?1 WHERE id=?2",params![message,job_id]);}
            }
        }
    });
    Ok(Json(json!({"id":id,"status":"generating"})))
}
async fn generate_image(key:&str,prompt:&str,details:&Value)->Result<String,String> {
    request_image(key,prompt,details,"https://api.openai.com/v1/images/generations").await
}
async fn request_image(key:&str,prompt:&str,details:&Value,endpoint:&str)->Result<String,String> {
    let full_prompt=format!("Design a finished, beautiful portrait wedding invitation, not a photograph of stationery. Follow the couple's creative direction: {prompt}\nWedding details (use these exact facts, do not invent names, dates, locations, or events): {details}\nIntegrate the provided details as carefully typeset, legible invitation text with elegant visual hierarchy. Add an appropriate short invitation phrase. No placeholder text, watermarks, mockup shadows or external frame. The image itself is the entire invitation. Prioritize the requested language, motifs, colors and composition.");
    let client=reqwest::Client::builder().timeout(Duration::from_secs(300)).build().map_err(|_|"Could not initialize the AI connection.".to_string())?;
    let response=client.post(endpoint).bearer_auth(key)
        .json(&json!({"model":model(),"prompt":full_prompt,"n":1,"size":"1024x1536","quality":"medium","output_format":"png"})).send().await
        .map_err(|e| if e.is_timeout(){"Generation timed out. Please try again.".to_string()}else{"Could not reach the image service. Check the server's internet connection.".to_string()})?;
    let status=response.status();
    if !status.is_success() {
        return Err(match status.as_u16(){
            401=>"The API key was rejected. Reconnect AI with a valid key.",
            403=>"This API project does not have access to the image model. Check project permissions or organization verification.",
            429=>"The API quota or rate limit was reached. Check billing and limits before trying again.",
            400=>"The image service could not accept this request. Try a different prompt and check the configured model.",
            _=>"The image service is temporarily unavailable. Please try again later."
        }.to_string());
    }
    let body:Value=response.json().await.map_err(|_|"The image service returned an unreadable response.".to_string())?;
    let b64=body.pointer("/data/0/b64_json").and_then(Value::as_str).ok_or("No image was returned. Please try again.")?;
    let bytes=STANDARD.decode(b64).map_err(|_|"The image response was invalid.")?;
    if bytes.len()>30*1024*1024 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {return Err("The image service returned an unsupported image.".into());}
    Ok(b64.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_marks_incomplete_jobs_failed() {
        let db=Connection::open_in_memory().unwrap(); initialize(&db).unwrap();
        db.execute("INSERT INTO invitations(id,prompt,details,status) VALUES('test','flowers','{}','generating')",[]).unwrap();
        initialize(&db).unwrap();
        let status:String=db.query_row("SELECT status FROM invitations WHERE id='test'",[],|r|r.get(0)).unwrap();
        assert_eq!(status,"failed");
    }
    #[tokio::test]
    async fn provider_request_sends_prompt_and_decodes_png() {
        let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr=listener.local_addr().unwrap();
        let mock=axum::Router::new().route("/images",axum::routing::post(|headers:axum::http::HeaderMap,Json(body):Json<Value>|async move {
            assert_eq!(headers["authorization"],"Bearer test-key");
            assert!(body["prompt"].as_str().unwrap().contains("watercolor jasmine"));
            assert!(body["prompt"].as_str().unwrap().contains("Asha & Dev"));
            assert_eq!(body["size"],"1024x1536");
            Json(json!({"data":[{"b64_json":STANDARD.encode(b"\x89PNG\r\n\x1a\nmock-image") }]}))
        }));
        let server=tokio::spawn(async move { axum::serve(listener,mock).await.unwrap() });
        let result=request_image("test-key","watercolor jasmine",&json!({"names":"Asha & Dev"}),&format!("http://{addr}/images")).await.unwrap();
        assert!(STANDARD.decode(result).unwrap().starts_with(b"\x89PNG"));
        server.abort();
    }
    #[tokio::test]
    async fn provider_rejection_does_not_expose_raw_response() {
        let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr=listener.local_addr().unwrap();
        let mock=axum::Router::new().route("/images",axum::routing::post(||async {(StatusCode::UNAUTHORIZED,"provider-debug-secret")}));
        let server=tokio::spawn(async move { axum::serve(listener,mock).await.unwrap() });
        let err=request_image("test-key","flowers",&json!({}),&format!("http://{addr}/images")).await.unwrap_err();
        assert!(err.contains("key was rejected")); assert!(!err.contains("provider-debug-secret"));
        server.abort();
    }
}
