use crate::url::parse_url;
use axum::extract::State;
use axum::response::Response;
use axum::{extract::Query, http::StatusCode, response::IntoResponse};
use hyper::header;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::debug;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct ProxyQuery {
    url: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    client: reqwest::Client,
    proxy_url: Arc<Option<Url>>,
}

impl AppState {
    pub fn new(client: reqwest::Client, proxy_url: Option<Url>) -> Self {
        Self {
            client,
            proxy_url: Arc::new(proxy_url),
        }
    }
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    Query(params): Query<ProxyQuery>,
) -> impl IntoResponse {
    // Decode the URL parameter
    let decoded_url = match percent_decode_str(&params.url).decode_utf8() {
        Ok(s) => s.to_string(),
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid URL encoding").into_response(),
    };

    debug!("decoded_url: {}", decoded_url);

    let requested_url = match parse_url(&decoded_url) {
        Ok(url) => url,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid URL").into_response(),
    };

    debug!("requested_url: {}", requested_url);

    let client = state.client.clone();

    if let Some(proxy_url) = state.proxy_url.as_ref() {
        let proxy_url = proxy_url.clone();
        if proxy_url.authority() != requested_url.authority() {
            return (StatusCode::BAD_REQUEST, "Proxy URLs are restricted.").into_response();
        }
    }

    let response = match client.get(requested_url).send().await {
        Ok(resp) => resp,
        Err(_) => return (StatusCode::BAD_GATEWAY, "Failed to send request").into_response(),
    };

    // Get the response body and return it
    match response.text().await {
        Ok(body) => body.into_response(),
        Err(_) => (StatusCode::BAD_GATEWAY, "Failed to read response body").into_response(),
    }
}

#[axum::debug_handler]
pub async fn config_handler(State(state): State<AppState>) -> impl IntoResponse {
    let proxy_url = state.proxy_url.as_ref().clone();

    let js_code = format!(
        r#"
window.dashboardConfig = {{
  datasourceUrl: "{}",
}};
"#,
        proxy_url.map(|url| url.to_string()).unwrap_or_default()
    );

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/javascript")
        .body(js_code.to_string())
        .unwrap();

    response.into_response()
}

pub async fn shutdown_signal(handle: axum_server::Handle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Received termination signal shutting down");
    // 10 secs is how long docker will wait to force shutdown
    handle.graceful_shutdown(Some(Duration::from_secs(10)));
}
