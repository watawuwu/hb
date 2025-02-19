use anyhow::Result;
use clap::ValueEnum;
use prometheus_client::encoding::EncodeLabelValue;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use strum::AsRefStr;

#[derive(Debug)]
pub(crate) struct Client {
    underlying: reqwest::Client,
    raw_request: reqwest::Request,
}

type Status = u16;
type ResponseSize = u64;

impl Clone for Client {
    fn clone(&self) -> Self {
        let underlying = self.underlying.clone();
        // Since streams are not used, it will never be None
        let raw_request = self.raw_request.try_clone().unwrap();
        Self {
            underlying,
            raw_request,
        }
    }
}

impl Client {
    pub(crate) fn try_new(req: Request) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .timeout(req.timeout)
            .user_agent(format!("hb-client/{}", env!("CARGO_PKG_VERSION")))
            .danger_accept_invalid_certs(req.insecure)
            .use_rustls_tls()
            .tls_built_in_root_certs(true);

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-tool-name"),
            HeaderValue::from_static("hb-client"),
        );
        headers.insert(
            HeaderName::from_static("x-tool-version"),
            HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
        );
        builder = builder.default_headers(headers);

        if let (Some(ip), Some(domain)) = (req.resolve, req.url.domain()) {
            let socket = match req.url.port() {
                Some(port) => SocketAddr::new(ip, port),
                None => {
                    let port = if req.url.scheme() == "https" { 443 } else { 80 };
                    SocketAddr::new(ip, port)
                }
            };

            builder = builder.resolve(domain, socket);
        }

        if req.http_version == HttpVersion::Http2 {
            builder = builder.http2_prior_knowledge();
        }

        if req.disable_keepalive {
            builder = builder.pool_max_idle_per_host(0);
        }

        if let Some(root_cert) = &req.root_cert {
            builder = builder.add_root_certificate(reqwest::Certificate::from_pem(
                std::fs::read(root_cert)?.as_slice(),
            )?);
        }

        let client = builder.build()?;

        let underlying = Self::builder(client, req)?;
        let (underlying, raw_request) = underlying.build_split();
        let raw_request = raw_request?;

        Ok(Self {
            underlying,
            raw_request,
        })
    }

    fn builder(underlying: reqwest::Client, req: Request) -> Result<reqwest::RequestBuilder> {
        let mut builder = underlying.request(req.method.into(), req.url);

        if !req.body.is_empty() {
            builder = builder.body(req.body);
        }

        for (key, value) in req.headers.iter() {
            builder = builder.header(key, value);
        }

        if req.disable_keepalive && req.http_version == HttpVersion::Http11 {
            builder = builder.header("Connection", "close");
        }

        if let Some((username, password)) = &req.basic_auth {
            builder = builder.basic_auth(username, Some(password));
        }

        Ok(builder)
    }

    // Emulate the size of the header
    // Notes: This is not the exact size, but an approximation. ex) HTTP/1.1 200 OK\r\n is not included.
    async fn calculate_header_size(headers: &reqwest::header::HeaderMap) -> u64 {
        let mut size = headers.iter().fold(0, |mut acc, (name, value)| {
            acc += name.as_str().len() + 2; // Header name + ": "
            if let Ok(v) = value.to_str() {
                acc += v.len();
            }
            acc += 2; // \r\n
            acc
        });

        // Add final \r\n that marks end of headers
        size += 2;

        size as u64
    }

    pub(crate) async fn request(self) -> Result<(Status, ResponseSize)> {
        let resp = self.underlying.execute(self.raw_request).await?;

        let status = resp.status().as_u16();
        let header_size = Self::calculate_header_size(resp.headers()).await;
        let body_size = resp.bytes().await?.len() as u64;

        Ok((status, header_size + body_size))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum, AsRefStr, Hash, EncodeLabelValue)]
#[strum(serialize_all = "UPPERCASE")]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Options,
    Trace,
    Patch,
}

impl From<Method> for reqwest::Method {
    fn from(item: Method) -> Self {
        reqwest::Method::from_str(item.as_ref()).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, ValueEnum, AsRefStr)]
pub enum HttpVersion {
    #[clap(name = "1.1")]
    Http11,
    #[clap(name = "2")]
    Http2,
}

impl From<HttpVersion> for reqwest::Version {
    fn from(item: HttpVersion) -> Self {
        match item {
            HttpVersion::Http11 => reqwest::Version::HTTP_11,
            HttpVersion::Http2 => reqwest::Version::HTTP_2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub http_version: HttpVersion,
    pub url: url::Url,
    pub method: Method,
    pub headers: HashMap<String, String>,
    pub timeout: Duration,
    pub body: Vec<u8>,
    pub basic_auth: Option<(String, String)>,
    pub insecure: bool,
    pub disable_keepalive: bool,
    pub root_cert: Option<PathBuf>,
    pub resolve: Option<IpAddr>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    #[tokio::test]
    async fn test_calculate_header_size() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            HeaderName::from_static("content-length"),
            HeaderValue::from_static("42"),
        );
        headers.insert(
            HeaderName::from_static("x-custom-header"),
            HeaderValue::from_static("test-value"),
        );

        let size = Client::calculate_header_size(&headers).await;

        // Size breakdown:
        // "content-type: application/json\r\n" (32)
        // "content-length: 42\r\n" (20)
        // "x-custom-header: test-value\r\n" (29)
        // Final "\r\n" (2)
        assert_eq!(size, 83);
    }

    #[tokio::test]
    async fn test_calculate_header_size_with_empty_headers() {
        let headers = HeaderMap::new();
        let size = Client::calculate_header_size(&headers).await;

        // Only final "\r\n"
        assert_eq!(size, 2);
    }

    #[tokio::test]
    async fn test_calculate_header_size_with_invalid_value() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-binary"),
            HeaderValue::from_bytes(&[0xFF, 0xFE, 0xFD]).unwrap(),
        );

        let size = Client::calculate_header_size(&headers).await;

        // Invalid header values are ignored, only name and delimiters are counted
        // "x-binary: \r\n" (12)
        // Final "\r\n" (2)
        assert_eq!(size, 14);
    }

    async fn setup_test_client(server: &Server, path: &str, method: Method) -> Result<Client> {
        let url = server.url() + path;
        let request = Request {
            http_version: HttpVersion::Http11,
            url: url::Url::parse(&url)?,
            method,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            body: Vec::new(),
            basic_auth: None,
            insecure: false,
            disable_keepalive: false,
            root_cert: None,
            resolve: None,
        };
        Client::try_new(request)
    }

    #[tokio::test]
    async fn test_successful_get_request() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/test")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("Hello, World!")
            .create_async()
            .await;

        let client = setup_test_client(&server, "/test", Method::Get).await?;
        let (status, size) = client.request().await?;

        assert_eq!(status, 200);
        // header size ( "connection": "close", "content-type": "text/plain", "content-length": "13", "date": "Sun, 16 Feb 2025 04:33:05 GMT") + body size ("Hello, World!")
        assert_eq!(size, 117);
        mock.assert_async().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_not_found_request() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/notfound")
            .with_status(404)
            .create_async()
            .await;

        let client = setup_test_client(&server, "/notfound", Method::Get).await?;
        let (status, size) = client.request().await?;

        assert_eq!(status, 404);
        // header size ( "connection": "close", "content-type": "text/plain", "content-length": "0", "date": "Sun, 16 Feb 2025 04:33:05 GMT") + body size ("")
        assert_eq!(size, 77);
        mock.assert_async().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_post_request_with_body() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/submit")
            .match_body("test data")
            .with_body("test data")
            .with_status(201)
            .create_async()
            .await;

        let request = Request {
            http_version: HttpVersion::Http11,
            url: url::Url::parse(&(server.url() + "/submit"))?,
            method: Method::Post,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            body: "test data".as_bytes().to_vec(),
            basic_auth: None,
            insecure: false,
            disable_keepalive: false,
            root_cert: None,
            resolve: None,
        };

        let client = Client::try_new(request)?;
        let (status, size) = client.request().await?;

        assert_eq!(status, 201);
        // header size ( "connection": "close", "content-type": "text/plain", "content-length": "9", "date": "Sun, 16 Feb 2025 04:33:05 GMT") + body size ("test data")
        assert_eq!(size, 86);
        mock.assert_async().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_client_builder_headers() -> Result<()> {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/")
            .match_header("x-tool-name", "hb-client")
            .match_header("x-tool-version", env!("CARGO_PKG_VERSION"))
            .with_status(200)
            .create_async()
            .await;

        let request = Request {
            http_version: HttpVersion::Http11,
            url: url::Url::parse(&server.url())?,
            method: Method::Get,
            headers: HashMap::new(),
            timeout: Duration::from_secs(30),
            body: Vec::new(),
            basic_auth: None,
            insecure: false,
            disable_keepalive: false,
            root_cert: None,
            resolve: None,
        };

        let client = Client::try_new(request)?;
        let (status, _) = client.request().await?;

        assert_eq!(status, 200);

        mock.assert_async().await;

        Ok(())
    }
}
