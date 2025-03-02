use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use clap::builder::{styling, Styles};
use clap::Parser;
use std::fmt::Debug;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use url::Url;

const DEFAULT_PORT: u16 = 8080;
const DEFAULT_TLS_PORT: u16 = 8443;

#[derive(Parser, Debug)]
#[command(author, version, about, next_line_help = true, long_about = None, styles(help_styles()))]
pub struct Args {
    /// Port to listen on
    #[arg(short, long, env = "HB_DASHBOARD_PORT")]
    pub port: Option<u16>,

    /// Host to listen on
    #[arg(
        short = 'H',
        long,
        default_value = "0.0.0.0",
        env = "HB_DASHBOARD_HOST"
    )]
    pub host: String,

    /// Path to the dist directory
    #[arg(
        short,
        long,
        default_value = "frontend/dist",
        env = "HB_DASHBOARD_DIST_PATH"
    )]
    pub dist_path: PathBuf,

    /// Path to the TLS certificate file
    #[arg(short = 'c', long, env = "HB_DASHBOARD_TLS_CERT_FILE")]
    pub tls_cert_path: Option<PathBuf>,

    /// Path to the TLS key file
    #[arg(short = 'k', long, env = "HB_DASHBOARD_TLS_KEY_FILE")]
    pub tls_key_path: Option<PathBuf>,

    /// URL to datasource URL(Prometheus)
    #[arg(short = 'u', long, env = "HB_DASHBOARD_DATASOURCE_URL")]
    pub datasource_url: Option<Url>,
}

impl Args {
    pub async fn server_config(&self) -> Result<(SocketAddr, Option<RustlsConfig>)> {
        let socket_addr = self.socket_addr()?;
        let tls_config = self.tls_config().await;

        Ok((socket_addr, tls_config))
    }

    fn socket_addr(&self) -> Result<SocketAddr> {
        let ipv4: Ipv4Addr = self.host.parse()?;
        let port = match (self.port, &self.tls_cert_path, &self.tls_key_path) {
            (Some(port), _, _) => port,
            (None, Some(_), Some(_)) => DEFAULT_TLS_PORT,
            _ => DEFAULT_PORT,
        };
        Ok((ipv4, port).into())
    }

    async fn tls_config(&self) -> Option<RustlsConfig> {
        match (&self.tls_cert_path, &self.tls_key_path) {
            (Some(cert_path), Some(key_path)) => {
                RustlsConfig::from_pem_file(cert_path, key_path).await.ok()
            }
            _ => None,
        }
    }
}

fn help_styles() -> Styles {
    styling::Styles::styled()
        .header(styling::AnsiColor::Green.on_default() | styling::Effects::BOLD)
        .usage(styling::AnsiColor::Green.on_default() | styling::Effects::BOLD)
        .literal(styling::AnsiColor::Blue.on_default() | styling::Effects::BOLD)
        .placeholder(styling::AnsiColor::Cyan.on_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}
