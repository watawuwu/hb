use crate::http::Request;
use crate::reporter::ExecMode;
use crate::reporter::REPORT_INTERVAL;
use crate::reporter::exporter::StdoutExporter;
use crate::reporter::formatter::OutputFormat;
use crate::time::now_ts;
use anyhow::{Result, bail};
use clap::ValueEnum;
use opentelemetry::{
    InstrumentationScope, KeyValue, global,
    metrics::{Counter, Gauge, Histogram},
};
use opentelemetry_otlp::{MetricExporter, Protocol, WithExportConfig};
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::{
    Resource,
    metrics::{SdkMeterProvider, periodic_reader_with_async_runtime::PeriodicReader},
};
use std::time::Duration;
use tracing::*;

const OTLP_SERVICE_NAME: &str = "hb";

#[derive(Debug, Clone, ValueEnum)]
pub enum OtlpProtocol {
    Grpc,
    HttpBinary,
    HttpJson,
}

impl From<OtlpProtocol> for Protocol {
    fn from(protocol: OtlpProtocol) -> Self {
        match protocol {
            OtlpProtocol::Grpc => Protocol::Grpc,
            OtlpProtocol::HttpBinary => Protocol::HttpBinary,
            OtlpProtocol::HttpJson => Protocol::HttpJson,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Metrics {
    start_time: Gauge<f64>,
    duration_seconds: Histogram<f64>,
    response_size_bytes: Histogram<u64>,
    error_total: Counter<u64>,
    method: String,
    path: String,
}

impl Metrics {
    pub fn new(
        start_time: Gauge<f64>,
        duration_seconds: Histogram<f64>,
        response_size_bytes: Histogram<u64>,
        error_total: Counter<u64>,
        method: String,
        path: String,
    ) -> Self {
        Self {
            start_time,
            duration_seconds,
            response_size_bytes,
            error_total,
            method,
            path,
        }
    }

    pub fn record_start_time(&self) {
        let now = now_ts();
        self.start_time.record(now, &[]);
    }

    pub fn record_duration(&self, duration: f64, status: u16) {
        self.duration_seconds.record(
            duration,
            &[
                KeyValue::new("method", self.method.clone()),
                KeyValue::new("path", self.path.clone()),
                KeyValue::new("status", status.to_string()),
            ],
        );
    }

    pub fn record_response_size(&self, size: u64) {
        self.response_size_bytes.record(size, &[]);
    }

    pub fn record_error(&self, phase: &str) {
        self.error_total
            .add(1, &[KeyValue::new("phase", phase.to_string())]);
    }
}

#[derive(Debug)]
pub struct OtlpOptions {
    pub endpoint: Option<url::Url>,
    pub protocol: OtlpProtocol,
    pub interval: Duration,
    pub requests: Option<u64>,
    pub duration: Option<Duration>,
    pub output_format: OutputFormat,
    pub no_interactive: bool,
    pub no_clear_console: bool,
}

pub async fn setup_metrics(
    opts: OtlpOptions,
    req: &Request,
) -> Result<(SdkMeterProvider, Metrics)> {
    let readers = create_periodic_readers(&opts)?;
    let provider = create_provider(readers);
    let metrics = create_metrics(req);

    Ok((provider, metrics))
}

fn create_periodic_readers(opts: &OtlpOptions) -> Result<Vec<PeriodicReader>> {
    let stdout_meter_interval = if opts.no_interactive {
        // If we're not in interactive mode, we don't need to update the console
        // NOTE: However, when the shutdown call is made, the display
        Duration::from_secs(u64::MAX)
    } else {
        REPORT_INTERVAL
    };

    let exec_mode = match (opts.requests, opts.duration) {
        (Some(num), None) => ExecMode::ByCount(num),
        (None, Some(duration)) => ExecMode::ByDuration(duration),
        _ => bail!("Either num or duration must be specified"),
    };
    let exporter = StdoutExporter::new(
        opts.output_format.to_owned(),
        exec_mode,
        opts.no_interactive,
        opts.no_clear_console,
    );

    let mut readers = Vec::new();
    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(stdout_meter_interval)
        .build();
    readers.push(reader);

    if let Some(endpoint) = opts.endpoint.clone() {
        let exporter = match opts.protocol {
            OtlpProtocol::Grpc => MetricExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .with_protocol(opts.protocol.clone().into())
                .with_timeout(Duration::from_secs(3))
                .build()?,
            OtlpProtocol::HttpBinary | OtlpProtocol::HttpJson => MetricExporter::builder()
                .with_http()
                .with_endpoint(endpoint)
                .with_protocol(opts.protocol.clone().into())
                .with_timeout(Duration::from_secs(3))
                .build()?,
        };

        let reader = PeriodicReader::builder(exporter, runtime::Tokio)
            .with_interval(opts.interval)
            .build();

        readers.push(reader);
    };

    Ok(readers)
}

fn create_provider(readers: Vec<PeriodicReader>) -> SdkMeterProvider {
    let resource = Resource::builder()
        .with_service_name(OTLP_SERVICE_NAME)
        .with_attribute(KeyValue::new(
            "service.instance.id",
            uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, OTLP_SERVICE_NAME.as_bytes())
                .to_string(),
        ))
        .build();

    let mut provider_builder = SdkMeterProvider::builder().with_resource(resource);

    for reader in readers {
        provider_builder = provider_builder.with_reader(reader);
    }
    let provider = provider_builder.build();

    debug!("provider: {:?}", provider);

    global::set_meter_provider(provider.clone());

    provider
}

fn create_metrics(req: &Request) -> Metrics {
    let scope = InstrumentationScope::builder("client")
        .with_version("0.1.0")
        .build();

    let duration_seconds = global::meter_with_scope(scope.clone())
        .f64_histogram("http_client_request_duration")
        .with_boundaries(vec![
            0.001, 0.005, 0.01, 0.025, 0.05, 0.07, 0.1, 0.25, 0.5, 1.0, 5.0, 10.0,
        ])
        .with_description("Histogram of latencies for HTTP client requests.")
        .with_unit("s")
        .build();

    let error_total = global::meter_with_scope(scope.clone())
        .u64_counter("http_client_errors")
        .with_description("Total number of HTTP client errors.")
        .build();

    let response_size_bytes = global::meter_with_scope(scope.clone())
        .u64_histogram("http_client_response_size")
        .with_boundaries(vec![0.0, 100.0, 1024.0, 1024.0 * 100.0, 1024.0 * 1024.0])
        .with_description("Histogram of response sizes for HTTP client requests.")
        .with_unit("bytes")
        .build();

    let start_time = global::meter_with_scope(scope.clone())
        .f64_gauge("http_client_start_time")
        .with_description("Start time of the HTTP client.")
        .with_unit("s")
        .build();

    Metrics::new(
        start_time,
        duration_seconds,
        response_size_bytes,
        error_total,
        req.method.as_ref().to_string(),
        req.url.path().to_string(),
    )
}
