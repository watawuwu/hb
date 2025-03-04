use anyhow::{Result, bail};
use clap::builder::{Styles, styling};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use hb::bench::BenchOption;
use hb::http::HttpVersion;
use hb::http::{Method, Request};
use hb::otlp::{OtlpOptions, OtlpProtocol};
use hb::reporter::formatter::OutputFormat;
use regex::Regex;
use std::ffi::OsString;
use std::fmt::{Debug, Display};
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, next_line_help = true, long_about = None, styles(help_styles()))]
pub struct Args {
    /// Number of requests to make
    #[arg(short = 'n', long, group = "load_params", value_parser = parse_positive::<u64>)]
    pub requests: Option<u64>,

    /// Duration of requests to make.
    /// Available units: ms, s, m, h, d
    #[arg(short, long, value_parser = parse_duration, group = "load_params")]
    pub duration: Option<Duration>,

    /// Number of clients to simulate
    #[arg(short, long, default_value = "10", value_parser = parse_positive::<usize>)]
    pub clients: usize,

    /// Number of native threads to use
    #[arg(short, long)]
    pub thread: Option<usize>,

    /// If server doesn't address http/2, it will be downgraded to http/1.1
    #[arg(long, default_value = "1.1")]
    pub http_version: HttpVersion,

    /// HTTP method to use
    #[arg(short, long, default_value = "get")]
    pub method: Method,

    /// Headers to include in the request
    #[arg(short = 'H', long, value_parser = parse_key_value)]
    pub headers: Vec<(String, String)>,

    /// Timeout for the request
    /// Available units: ms, s, m, h, d
    #[arg(long, value_parser = parse_duration, default_value = "3s")]
    pub timeout: Duration,

    /// Body of the request
    #[arg(short, long, group = "body_params")]
    pub body: Option<String>,

    /// File containing the body of the request
    #[arg(short = 'B', long, group = "body_params")]
    pub body_file: Option<PathBuf>,

    /// Basic auth in the format username:password
    #[arg(long, value_parser = parse_basic_auth)]
    pub basic_auth: Option<(String, String)>,

    /// OTLP endpoint to send metrics to
    #[arg(long)]
    pub otlp_endpoint: Option<url::Url>,

    /// OTLP endpoint to send metrics to
    #[arg(long, default_value = "http-json")]
    pub otlp_protocol: OtlpProtocol,

    /// Interval to send metrics to OTLP
    /// Available units: ms, s, m, h, d
    #[arg(long, value_parser = parse_duration, default_value = "10s")]
    pub otlp_interval: Duration,

    /// Skip certificate verification
    #[arg(long, default_value = "false")]
    pub insecure: bool,

    /// Disable keepalive
    #[arg(long, default_value = "false")]
    pub disable_keepalive: bool,

    /// Root certificate to use
    #[arg(long)]
    pub root_cert: Option<PathBuf>,

    /// IP address to resolve the hostname to
    #[arg(long)]
    pub resolve: Option<IpAddr>,

    /// Requests per second
    /// Experimental feature: this is not guaranteed to be accurate
    #[arg(long)]
    pub rps: Option<u64>,

    /// Output format
    #[arg(short, long = "output", default_value = "text")]
    pub output_format: OutputFormat,

    /// URL to make the request to
    #[arg(name = "URL")]
    pub url: url::Url,

    /// No-interactive mode
    #[arg(long)]
    pub no_interactive: bool,

    /// No-clear-console mode
    #[arg(long)]
    pub no_clear_console: bool,
}

impl Args {
    pub fn request(&self) -> Request {
        let body = match (self.body.clone(), self.body_file.clone()) {
            (Some(body), None) => body.as_bytes().to_vec(),
            (None, Some(body_file)) => std::fs::read(body_file).unwrap(),
            _ => Vec::new(),
        };

        Request {
            http_version: self.http_version.clone(),
            url: self.url.clone(),
            method: self.method.clone(),
            headers: self.headers.iter().cloned().collect(),
            timeout: self.timeout,
            body,
            basic_auth: self.basic_auth.clone(),
            insecure: self.insecure,
            disable_keepalive: self.disable_keepalive,
            root_cert: self.root_cert.clone(),
            resolve: self.resolve,
        }
    }

    pub fn otlp_options(&self) -> OtlpOptions {
        OtlpOptions {
            endpoint: self.otlp_endpoint.clone(),
            protocol: self.otlp_protocol.clone(),
            interval: self.otlp_interval,
            requests: self.requests,
            duration: self.duration,
            output_format: self.output_format.clone(),
            no_interactive: self.no_interactive,
            no_clear_console: self.no_clear_console,
        }
    }

    pub fn bench_options(&self) -> Result<BenchOption> {
        BenchOption::try_new(self.requests, self.duration, self.clients, self.rps)
    }

    pub fn parse_wrapper() -> Result<Args> {
        let args: Vec<_> = std::env::args().collect();
        Self::parse_wrapper_from(args)
    }

    pub fn parse_wrapper_from<I, T>(itr: I) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let args: Args = {
            let mut args = Args::parse_from(itr);

            // If output is json, disable interactive mode
            if args.output_format == OutputFormat::Json {
                args.no_interactive = true;
            };

            args
        };
        args.validation()?;
        Ok(args)
    }

    // This function is a wrapper around Args::parse() that performs additional validation and configuration
    fn validation(&self) -> Result<()> {
        if let Some(rps) = self.rps {
            if self.clients as u64 > rps {
                let mut cmd = Args::command();
                let err = cmd
                    .error(
                        ErrorKind::ArgumentConflict,
                        "RPS must be greater than or equal to the number of clients",
                    )
                    .into();
                return Err(err);
            }
        }

        if let Some(num) = self.requests {
            if self.clients as u64 > num {
                let mut cmd = Args::command();
                let err = cmd
                    .error(
                        ErrorKind::ArgumentConflict,
                        "Number of clients must be greater than or equal to the number of requests",
                    )
                    .into();
                return Err(err);
            }
        }

        if self.disable_keepalive && self.http_version == HttpVersion::Http2 {
            let mut cmd = Args::command();
            let err = cmd
                .error(
                    ErrorKind::ArgumentConflict,
                    "HTTP/2 does not support keepalive",
                )
                .into();
            return Err(err);
        }

        Ok(())
    }
}

fn help_styles() -> Styles {
    styling::Styles::styled()
        .header(styling::AnsiColor::Green.on_default() | styling::Effects::BOLD)
        .usage(styling::AnsiColor::Green.on_default() | styling::Effects::BOLD)
        .literal(styling::AnsiColor::Blue.on_default() | styling::Effects::BOLD)
        .placeholder(styling::AnsiColor::Cyan.on_default())
}

fn parse_basic_auth(input: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 2 {
        bail!("Basic auth must be in the format username:password");
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn parse_duration(input: &str) -> Result<Duration> {
    let re_validate = Regex::new(r"^(\d+(?:\.\d+)?(?:ms|s|m|h|d))+$")?;
    if !re_validate.is_match(input) {
        bail!("Invalid format");
    }

    let re = Regex::new(r"(?P<value>\d+)(?P<unit>(ms|s|m|h|d))")?;
    let mut total_duration = Duration::new(0, 0);

    if re.captures_iter(input).count() == 0 {
        bail!("Duration must be in the format <value><unit>. Available units are ms/s/m/h/d.");
    }

    for caps in re.captures_iter(input) {
        let value: u64 = caps["value"].parse()?;
        let unit = &caps["unit"];

        let duration = match unit {
            "ms" => Duration::from_millis(value),
            "s" => Duration::from_secs(value),
            "m" => Duration::from_secs(value * 60),
            "h" => Duration::from_secs(value * 60 * 60),
            "d" => Duration::from_secs(value * 60 * 60 * 24),
            _ => bail!("Available units are ms/s/m/h/d"),
        };

        total_duration += duration;
    }

    if total_duration.as_millis() == 0 {
        bail!("Duration must be greater than 0");
    }

    Ok(total_duration)
}

fn parse_key_value(s: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        bail!("Key-value pair must be in the format key:value");
    }
    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
}

fn parse_positive<T>(s: &str) -> Result<T>
where
    T: FromStr + PartialOrd + Display,
    <T as FromStr>::Err: std::fmt::Debug,
{
    let value: T = s
        .parse()
        .map_err(|e| anyhow::anyhow!("'{}' is not a valid number: {:?}", s, e))?;
    if value < T::from_str("1").unwrap() {
        bail!("'{}' must be greater than or equal to 1", s);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_basic_auth() {
        let input = "username:password";
        let result = parse_basic_auth(input).unwrap();
        assert_eq!(result, ("username".to_string(), "password".to_string()));
    }

    #[test]
    fn test_parse_basic_auth_invalid() {
        let input = "usernamepassword";
        let result = parse_basic_auth(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_duration() {
        let inputs = [
            ("500ms", Duration::from_millis(500)),
            ("30s", Duration::from_secs(30)),
            ("45m", Duration::from_secs(2700)),
            ("1h", Duration::from_secs(3600)),
            ("2d", Duration::from_secs(172800)),
            ("1h30m15s", Duration::from_secs(5415)),
        ];

        inputs.iter().for_each(|(input, expected)| {
            let result = parse_duration(input).unwrap();
            assert_eq!(result, *expected);
        });
    }

    #[test]
    fn test_parse_duration_invalid() {
        let inputs = ["1x", "-1s", "0s"];
        inputs.iter().for_each(|input| {
            let result = parse_duration(input);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_parse_key_value() {
        let input = "key:value";
        let result = parse_key_value(input).unwrap();
        assert_eq!(result, ("key".to_string(), "value".to_string()));
    }

    #[test]
    fn test_parse_key_value_invalid() {
        let input = "keyvalue";
        let result = parse_key_value(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_args_parsing() {
        let args = vec![
            "test",
            "--requests",
            "100",
            "--clients",
            "10",
            "--http-version",
            "2",
            "--method",
            "post",
            "--headers",
            "key:value",
            "--timeout",
            "5s",
            "--body",
            "test body",
            "--basic-auth",
            "user:pass",
            "--otlp-endpoint",
            "http://localhost:4317",
            "--otlp-interval",
            "15s",
            "--insecure",
            "--disable-keepalive",
            "--resolve",
            "127.0.0.1",
            "--rps",
            "50",
            "--output",
            "json",
            "http://example.com",
        ];
        let args = Args::parse_from(args);
        assert_eq!(args.requests, Some(100));
        assert_eq!(args.clients, 10);
        assert_eq!(args.http_version, HttpVersion::Http2);
        assert_eq!(args.method, Method::Post);
        assert_eq!(args.headers, vec![("key".to_string(), "value".to_string())]);
        assert_eq!(args.timeout, Duration::from_secs(5));
        assert_eq!(args.body, Some("test body".to_string()));
        assert_eq!(
            args.basic_auth,
            Some(("user".to_string(), "pass".to_string()))
        );
        assert_eq!(
            args.otlp_endpoint.unwrap().to_string(),
            "http://localhost:4317/"
        );
        assert_eq!(args.otlp_interval, Duration::from_secs(15));
        assert!(args.insecure);
        assert!(args.disable_keepalive);
        assert_eq!(args.resolve, Some(IpAddr::from_str("127.0.0.1").unwrap()));
        assert_eq!(args.rps, Some(50));
        assert_eq!(args.output_format, OutputFormat::Json);
        assert_eq!(args.url.to_string(), "http://example.com/");
    }

    #[test]
    fn test_parse_wrapper_valid_args() {
        let args = vec![
            "test",
            "--requests",
            "100",
            "--clients",
            "10",
            "http://example.com",
        ];
        let args = Args::parse_wrapper_from(args).unwrap();
        assert!(args.requests.is_some());
        assert_eq!(args.clients, 10);
    }

    #[test]
    fn test_parse_wrapper_invalid_rps() {
        let args = vec![
            "test",
            "--rps",
            "5",
            "--clients",
            "10", // clients > rps should fail
            "http://example.com",
        ];
        let err = Args::parse_wrapper_from(args);
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("RPS must be greater than or equal to the number of clients")
        );
    }

    #[test]
    fn test_parse_wrapper_invalid_requests() {
        let args = vec![
            "test",
            "--requests",
            "5",
            "--clients",
            "10", // clients > requests should fail
            "http://example.com",
        ];
        let err = Args::parse_wrapper_from(args);
        assert!(err.is_err());
        assert!(
            err.unwrap_err().to_string().contains(
                "Number of clients must be greater than or equal to the number of requests"
            )
        );
    }

    #[test]
    fn test_parse_wrapper_invalid_http2_keepalive() {
        let args = vec![
            "test",
            "--http-version",
            "2",
            "--disable-keepalive", // HTTP/2 with disable_keepalive should fail
            "http://example.com",
        ];
        let err = Args::parse_wrapper_from(args);
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("HTTP/2 does not support keepalive")
        );
    }

    #[test]
    fn test_parse_wrapper_json_output_disables_interactive() {
        let args = vec![
            "test",
            "--requests",
            "10",
            "--output",
            "json",
            "http://example.com",
        ];
        let args = Args::parse_wrapper_from(args).unwrap();
        assert!(args.no_interactive);
    }
}
