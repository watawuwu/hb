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
use url::Url;

#[derive(Debug, Deserialize)]
pub struct ProxyQuery {
    url: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    client: reqwest::Client,
    datasource_url: Arc<Option<Url>>,
}

impl AppState {
    pub fn new(client: reqwest::Client, datasource_url: Option<Url>) -> Self {
        Self {
            client,
            datasource_url: Arc::new(datasource_url),
        }
    }
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    Query(params): Query<ProxyQuery>,
) -> impl IntoResponse {
    let requested_url = match decode_and_parse_url(&params.url) {
        Ok(url) => url,
        Err(err_msg) => return (StatusCode::BAD_REQUEST, err_msg).into_response(),
    };

    if !validate_datasource_url(&state, &requested_url) {
        return (StatusCode::BAD_REQUEST, "Datasource URLs are restricted.").into_response();
    }

    match send_request(&state.client, requested_url).await {
        Ok(body) => body.into_response(),
        Err(err_msg) => (StatusCode::BAD_GATEWAY, err_msg).into_response(),
    }
}

fn decode_and_parse_url(url: &str) -> Result<Url, &'static str> {
    let decoded_url = percent_decode_str(url).decode_utf8().map_err(|e| {
        tracing::debug!("Invalid URL encoding: {}", e);
        "Invalid URL encoding"
    })?;
    parse_url(&decoded_url).map_err(|e| {
        tracing::debug!("Invalid URL: {}", e);
        "Invalid URL"
    })
}

fn validate_datasource_url(state: &AppState, requested_url: &Url) -> bool {
    if let Some(datasource_url) = state.datasource_url.as_ref() {
        if datasource_url.authority() != requested_url.authority() {
            return false;
        }
    }
    true
}

async fn send_request(client: &reqwest::Client, url: Url) -> Result<String, &'static str> {
    let response = client.get(url).send().await.map_err(|e| {
        tracing::debug!("Failed to send request error: {}", e);
        "Failed to send request"
    })?;
    response.text().await.map_err(|e| {
        tracing::debug!("Failed to read response error: {}", e);
        "Failed to read response body"
    })
}

pub async fn config_handler(State(state): State<AppState>) -> impl IntoResponse {
    let datasource_url = state.datasource_url.as_ref().clone();

    let js_code = format!(
        r#"
window.dashboardConfig = {{
  datasourceUrl: "{}",
}};
"#,
        datasource_url
            .map(|url| url.to_string())
            .unwrap_or_default()
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
