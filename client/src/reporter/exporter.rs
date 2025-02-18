use crate::reporter::components::Reporter;
use crate::reporter::formatter::OutputFormat;
use crate::reporter::metrics::{
    ErrorMetrics, RequestDurationSecondsMetrics, ResponseSizeBytesMetrics, StartTimeMetrics,
};
use crate::reporter::ExecMode;
use crate::time::now_ts;
use anyhow::Result;
use async_trait::async_trait;
use atomic_float::AtomicF64;
use crossterm::queue;
use crossterm::style::Print;
use crossterm::{
    cursor,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::metrics::{
    data::ResourceMetrics, exporter::PushMetricExporter, Temporality,
};
use std::io::stdout;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct StdoutExporter {
    output_format: OutputFormat,
    exec_mode: ExecMode,
    no_interactive: bool,
    no_clear_console: bool,
    last_count: Arc<AtomicU64>,
    last_ts: Arc<AtomicF64>,
    last_report: Arc<Mutex<String>>,
}

impl StdoutExporter {
    pub(crate) fn new(
        output_format: OutputFormat,
        exec_mode: ExecMode,
        no_interactive: bool,
        no_clear_console: bool,
    ) -> Self {
        Self {
            output_format,
            exec_mode,
            no_interactive,
            no_clear_console,
            last_count: Arc::new(AtomicU64::new(0)),
            last_ts: Arc::new(AtomicF64::new(0.0)),
            last_report: Arc::new(Mutex::new(String::new())),
        }
    }

    fn update_last_report(&self, report: String) {
        *self.last_report.lock().unwrap() = report;
    }

    fn update_last_count(&self, count: u64) {
        self.last_count.store(count, Relaxed);
    }

    fn update_last_ts(&self) {
        self.last_ts.store(now_ts(), Relaxed);
    }

    async fn report(&self, meter: &mut ResourceMetrics) -> Result<String> {
        let last_cnt = self.last_count.load(Relaxed);
        let last_ts = self.last_ts.load(Relaxed);

        let meter_opt = RequestDurationSecondsMetrics::try_find(meter, last_cnt, last_ts)?;
        let duration_meter = match meter_opt {
            Some(duration_meter) => duration_meter,
            None => return Ok(String::from("Wait until metrics can be collected.")),
        };

        let meter_opt = StartTimeMetrics::find(meter);
        let start_meter = match meter_opt {
            Some(start_meter) => start_meter,
            None => return Ok(String::from("Wait until metrics can be collected.")),
        };

        let meter_opt = ResponseSizeBytesMetrics::find(meter);
        let resp_size_meter = match meter_opt {
            Some(resp_size_meter) => resp_size_meter,
            None => return Ok(String::from("Wait until metrics can be collected.")),
        };

        let err_meter = ErrorMetrics::find(meter);

        let reporter = Reporter::new(
            &duration_meter,
            &start_meter,
            &resp_size_meter,
            err_meter.as_ref(),
            &self.exec_mode,
        );

        let header = self.header(&reporter);
        let body = self.body(reporter)?;
        let report = header.clone().unwrap_or_default() + &body;

        self.update_last_report(body);
        self.update_last_count(duration_meter.count());
        self.update_last_ts();

        Ok(report)
    }

    fn header(&self, reporter: &Reporter) -> Option<String> {
        match (
            &self.output_format,
            self.no_interactive,
            self.no_clear_console,
        ) {
            (OutputFormat::Text, false, false) => Some(reporter.progress_bar()),
            (OutputFormat::Text, false, true) => {
                let rule = reporter.horizontal_rule();
                let bar = reporter.progress_bar();
                Some(format!("{}\n{}", rule, bar))
            }
            (OutputFormat::Text, _, _) => None,
            (OutputFormat::Json, _, _) => None,
        }
    }

    fn body(&self, reporter: Reporter) -> Result<String> {
        match self.output_format {
            OutputFormat::Text => reporter.text(),
            OutputFormat::Json => reporter.json(),
        }
    }

    fn print(&self, report: String) -> Result<()> {
        let mut stdout = stdout();

        if !self.no_clear_console {
            queue!(stdout, EnterAlternateScreen)?;
            let clear = Clear(ClearType::FromCursorDown);
            queue!(stdout, cursor::RestorePosition, clear)?;
        }

        queue!(stdout, Print(report))?;

        stdout.flush()?;
        Ok(())
    }

    fn last_print(&self) -> Result<()> {
        let mut stdout = stdout();
        // Output the last recorded report to the original buffer screen
        queue!(stdout, LeaveAlternateScreen)?;
        queue!(stdout, Print(self.last_report.lock().unwrap()))?;
        Ok(stdout.flush()?)
    }
}

#[async_trait]
impl PushMetricExporter for StdoutExporter {
    async fn export(&self, metrics: &mut ResourceMetrics) -> OTelSdkResult {
        let report = self
            .report(metrics)
            .await
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        self.print(report)
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))?;

        Ok(())
    }

    async fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }

    fn temporality(&self) -> Temporality {
        Temporality::Cumulative
    }

    // This function is called after all queues have been flushed,
    // so it is guaranteed to output the last recorded report
    fn shutdown(&self) -> OTelSdkResult {
        self.last_print()
            .map_err(|e| OTelSdkError::InternalFailure(e.to_string()))
    }
}
