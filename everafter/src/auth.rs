use axum::{extract::{Request, State}, http::{header, HeaderValue, StatusCode}, middleware::Next, response::{IntoResponse, Response}};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;

pub struct Config { expected: Option<Vec<u8>> }
impl Config {
    pub fn from_env() -> Self {
        let expected=std::env::var("EVERAFTER_TEST_PASSWORD").ok().filter(|p|!p.is_empty()).map(|password| {
            let username=std::env::var("EVERAFTER_TEST_USER").unwrap_or_else(|_|"tester".into());
            format!("{username}:{password}").into_bytes()
        });
        Self { expected }
    }
    fn accepts(&self, authorization: Option<&str>) -> bool {
        let Some(expected)=&self.expected else {return true};
        let Some(value)=authorization.and_then(|s|s.strip_prefix("Basic ")).and_then(|s|STANDARD.decode(s).ok()) else {return false};
        let mut difference=expected.len()^value.len();
        for (i,byte) in expected.iter().enumerate() {difference|=usize::from(*byte ^ value.get(i).copied().unwrap_or(0));}
        difference==0
    }
}
pub async fn protect(State(config):State<Arc<Config>>,request:Request,next:Next)->Response {
    if !config.accepts(request.headers().get(header::AUTHORIZATION).and_then(|h|h.to_str().ok())) {
        return (StatusCode::UNAUTHORIZED,[(header::WWW_AUTHENTICATE,"Basic realm=\"Everafter private beta\", charset=\"UTF-8\""),(header::CACHE_CONTROL,"no-store")],"This test site is password protected.").into_response();
    }
    let mut response=next.run(request).await;
    response.headers_mut().insert("x-robots-tag",HeaderValue::from_static("noindex, nofollow, noarchive"));
    response.headers_mut().insert("referrer-policy",HeaderValue::from_static("no-referrer"));
    response.headers_mut().insert("x-content-type-options",HeaderValue::from_static("nosniff"));
    if config.expected.is_some() {response.headers_mut().insert(header::CACHE_CONTROL,HeaderValue::from_static("no-store"));}
    response
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn beta_password_required_for_every_request() {
        let config=Config{expected:Some(b"tester:example-secret".to_vec())};
        assert!(!config.accepts(None));assert!(!config.accepts(Some("Bearer anything")));
        assert!(!config.accepts(Some(&format!("Basic {}",STANDARD.encode("tester:wrong")))));
        assert!(config.accepts(Some(&format!("Basic {}",STANDARD.encode("tester:example-secret")))));
    }
}
