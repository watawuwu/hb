mod args;
mod server;
mod url;

use anyhow::Result;
use args::Args;
use axum::{Router, routing::get};
use clap::Parser;
use server::{AppState, config_handler, proxy_handler, shutdown_signal};
use tower_http::services::ServeDir;
use tracing::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    info!("Args: {:#?}", args);

    let serve_dir = ServeDir::new(&args.dist_path);

    let client = reqwest::Client::new();
    let app_state = AppState::new(client, args.datasource_url.clone());

    let handle = axum_server::Handle::new();
    let shutdown_signal = shutdown_signal(handle.clone());
    tokio::spawn(shutdown_signal);

    // Define the router
    let app = Router::new()
        // /proxy for proxy request handler
        .route("/proxy", get(proxy_handler))
        // /config for dashboard config
        .route("/config.js", get(config_handler))
        // Serve static files
        // Nesting at the root is no longer supported. So use fallback_service instead.
        .fallback_service(serve_dir)
        // Add the app state to the router
        .with_state(app_state);

    // Start the server
    let (addr, tls_config) = args.server_config().await?;
    info!("Server running at {addr}");

    match tls_config {
        Some(tls_config) => {
            axum_server::bind_rustls(addr, tls_config)
                .handle(handle)
                .serve(app.into_make_service())
                .await?;
        }
        None => {
            axum_server::bind(addr)
                .handle(handle)
                .serve(app.into_make_service())
                .await?;
        }
    }

    Ok(())
}
