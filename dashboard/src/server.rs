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
    datasource_url: Arc<Url>,
}

impl AppState {
    pub fn new(client: reqwest::Client, datasource_url: Url) -> Self {
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

    if !validate_datasource_url(state.datasource_url.as_ref(), &requested_url) {
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

fn validate_datasource_url(datasource_url: &Url, requested_url: &Url) -> bool {
    datasource_url.authority() == requested_url.authority()
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use http_body_util::BodyExt;
    use mockito::Server;
    use reqwest::Client;
    use url::Url;

    #[tokio::test]
    async fn test_decode_and_parse_url_valid() {
        let url = "http%3A%2F%2Fexample.com";
        let result = decode_and_parse_url(url);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_decode_and_parse_url_invalid() {
        let url = "%E0%A4%A"; // Invalid percent encoding
        let result = decode_and_parse_url(url);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_datasource_url() {
        let datasource_url = Url::parse("http://example.com").unwrap();
        let requested_url = Url::parse("http://example.com").unwrap();
        assert!(validate_datasource_url(&datasource_url, &requested_url));
    }

    #[tokio::test]
    async fn test_proxy_handler_valid_url() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_body("Hello, world!")
            .create_async()
            .await;

        let client = Client::new();
        let datasource_url = Url::parse(&server.url())?;
        let state = AppState::new(client, datasource_url);
        let params = ProxyQuery { url: server.url() };

        let response = proxy_handler(State(state), Query(params))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);
        mock.assert_async().await;

        Ok(())
    }

    #[tokio::test]
    async fn test_proxy_handler_invalid_url() -> Result<()> {
        let client = Client::new();
        let datasource_url = Url::parse("http://example.com").unwrap();
        let state = AppState::new(client, datasource_url);
        let params = ProxyQuery {
            url: "invalid-url".to_string(),
        };

        let response = proxy_handler(State(state), Query(params))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[tokio::test]
    async fn test_proxy_handler_restricted_datasource_url() -> Result<()> {
        let client = Client::new();
        let datasource_url = Url::parse("http://restricted.example.com")?;
        let state = AppState::new(client, datasource_url);
        let params = ProxyQuery {
            url: "http://www.example.com".to_string(),
        };

        let response = proxy_handler(State(state), Query(params))
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        Ok(())
    }

    #[tokio::test]
    async fn test_config_handler() -> Result<()> {
        let client = Client::new();
        let datasource_url = Url::parse("http://example.com")?;
        let state = AppState::new(client, datasource_url);

        let response = config_handler(State(state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body();
        let bytes = body.collect().await?.to_bytes();
        let body_str = std::str::from_utf8(&bytes)?;

        assert!(body_str.contains(r#"window.dashboardConfig"#));
        assert!(body_str.contains(r#"datasourceUrl: "http://example.com/""#));

        Ok(())
    }
}
