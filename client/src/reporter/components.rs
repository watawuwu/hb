use crate::reporter::ExecMode;
use crate::reporter::formatter::{format_dynamic_precision, format_iec, format_percent};
use crate::reporter::metrics::{
    ErrorMetrics, RequestDurationSecondsMetrics, ResponseSizeBytesMetrics, StartTimeMetrics,
};
use anyhow::Result;
use serde::Serialize;
use tabled::settings::object::{Cell, Rows};
use tabled::settings::{Alignment, Border, Style};
use tabled::{Table, Tabled};

const TITLE_PADDING_SPACES: usize = 13;

#[derive(Debug, Serialize)]
pub(crate) struct Reporter {
    #[serde(skip)]
    pub bar: ProgressBar,
    pub summary: SummaryReport,
    pub counter: CounterReport,
    pub duration: DurationReport,
    pub response_size: ResponseSizeReport,
    pub errors: Vec<ErrorReport>,
    #[serde(skip)]
    horizontal_rule: HorizontalRule,
}

impl Reporter {
    pub(crate) fn new(
        duration_meter: &RequestDurationSecondsMetrics,
        start_meter: &StartTimeMetrics,
        resp_size_meter: &ResponseSizeBytesMetrics,
        error_meter: Option<&ErrorMetrics>,
        kind: &ExecMode,
    ) -> Self {
        let bar = ProgressBar::from_kinds(start_meter.elapsed_time(), duration_meter.count(), kind);
        let summary = SummaryReport::from(duration_meter);
        let counter = CounterReport::from(duration_meter);
        let duration = DurationReport::from(duration_meter);
        let response_size = ResponseSizeReport::from(resp_size_meter);
        let errors = error_meter.map_or_else(Vec::new, Vec::<ErrorReport>::from);

        Self {
            bar,
            summary,
            counter,
            duration,
            response_size,
            errors,
            horizontal_rule: HorizontalRule::new("─", 80),
        }
    }

    pub(crate) fn progress_bar(&self) -> String {
        self.bar.to_string()
    }

    pub(crate) fn horizontal_rule(&self) -> String {
        self.horizontal_rule.to_string()
    }

    pub(crate) fn text(self) -> Result<String> {
        let mut reports = vec![
            table(&[self.summary])?,
            table(&[self.counter])?,
            table(&[self.duration])?,
            table(&[self.response_size])?,
        ];

        if !self.errors.is_empty() {
            reports.push(table(&self.errors)?);
        }

        // add empty line
        reports.push(String::new());

        Ok(reports.join("\n"))
    }

    pub(crate) fn json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(&self)?)
    }
}

#[derive(Debug)]
pub struct ProgressBar {
    current: u64,
    current_display: String,
    target: u64,
    target_display: String,
}

impl ProgressBar {
    const BAR_CHAR_COUNT: usize = 50;
}

impl std::fmt::Display for ProgressBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // clip current to target
        let current = self.current.min(self.target);
        let ratio = current as f64 / self.target as f64;

        let filled_len = (Self::BAR_CHAR_COUNT as f64 * ratio).round() as usize;
        let empty_len = Self::BAR_CHAR_COUNT - filled_len;

        let bar = format!(
            "  {:>5}/{:<5} [{}>{}] {:.1}%\n",
            self.current_display,
            self.target_display,
            "=".repeat(filled_len),
            " ".repeat(empty_len),
            ratio * 100.0
        );
        write!(f, "{}", bar)
    }
}

impl ProgressBar {
    fn new(current: u64, current_display: String, target: u64, target_display: String) -> Self {
        Self {
            current,
            current_display,
            target,
            target_display,
        }
    }

    fn from_kinds(elapsed: u64, count: u64, kind: &ExecMode) -> Self {
        let (current, current_display, target, target_display) =
            kind.state_progress(elapsed, count);
        Self::new(current, current_display, target, target_display)
    }
}

#[derive(Debug)]
pub struct HorizontalRule {
    border: String,
    repeat: u8,
}

impl HorizontalRule {
    fn new(boarder: &str, repeat: u8) -> Self {
        Self {
            border: boarder.to_owned(),
            repeat,
        }
    }
}

impl std::fmt::Display for HorizontalRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let docorate = self.border.repeat(self.repeat as usize);
        write!(
            f,
            "{}\n \u{276F}\u{276F} {}\n{}\n",
            docorate,
            chrono::Local::now().to_rfc3339(),
            docorate
        )
    }
}

#[derive(Debug, Tabled, Serialize)]
#[tabled(rename_all = "Pascal")]
pub(crate) struct SummaryReport {
    #[tabled(rename = "Summary")]
    #[serde(skip)]
    _title: String,
    #[tabled(display = "format_percent", rename = "2xx|3xx Rate")]
    http_success_rate: f64,
    #[tabled(rename = "RPS")]
    #[serde(skip)]
    rps: u64,
}

impl From<&RequestDurationSecondsMetrics> for SummaryReport {
    fn from(item: &RequestDurationSecondsMetrics) -> Self {
        Self {
            _title: dummy_title(),
            http_success_rate: item.http_success_rate(),
            rps: item.rps(),
        }
    }
}

#[derive(Debug, Tabled, Serialize)]
#[tabled(rename_all = "Pascal")]
pub(crate) struct CounterReport {
    #[tabled(rename = "Counter")]
    #[serde(skip)]
    _title: String,
    #[tabled(rename = "2xx")]
    #[serde(rename = "2xx")]
    _2xx: u64,
    #[tabled(rename = "3xx")]
    #[serde(rename = "3xx")]
    _3xx: u64,
    #[tabled(rename = "4xx")]
    #[serde(rename = "4xx")]
    _4xx: u64,
    #[tabled(rename = "5xx")]
    #[serde(rename = "5xx")]
    _5xx: u64,
    total: u64,
}

impl From<&RequestDurationSecondsMetrics> for CounterReport {
    fn from(item: &RequestDurationSecondsMetrics) -> Self {
        Self {
            _title: dummy_title(),
            _2xx: item.status_2xx_count(),
            _3xx: item.status_3xx_count(),
            _4xx: item.status_4xx_count(),
            _5xx: item.status_5xx_count(),
            total: item.count(),
        }
    }
}

#[derive(Debug, Tabled, Serialize)]
#[tabled(rename_all = "Pascal")]
pub(crate) struct DurationReport {
    #[tabled(rename = "Duration")]
    #[serde(skip)]
    _title: String,
    #[tabled(display = "format_dynamic_precision")]
    mean: f64,
    #[tabled(display = "format_dynamic_precision")]
    p50: f64,
    #[tabled(display = "format_dynamic_precision")]
    p95: f64,
    #[tabled(display = "format_dynamic_precision")]
    p99: f64,
    #[tabled(display = "format_dynamic_precision")]
    min: f64,
    #[tabled(display = "format_dynamic_precision")]
    max: f64,
}

impl From<&RequestDurationSecondsMetrics> for DurationReport {
    fn from(item: &RequestDurationSecondsMetrics) -> Self {
        let a = 1.0;
        let _b = format_dynamic_precision(&a);
        DurationReport {
            _title: dummy_title(),
            mean: item.mean(),
            p50: item.p50(),
            p95: item.p95(),
            p99: item.p99(),
            min: item.min(),
            max: item.max(),
        }
    }
}

#[derive(Debug, Tabled, Serialize)]
#[tabled(rename_all = "Pascal")]
pub(crate) struct ResponseSizeReport {
    #[tabled(rename = "Response Size")]
    #[serde(skip)]
    _title: String,
    #[tabled(display = "format_iec")]
    mean: u64,
    #[tabled(display = "format_iec")]
    total: u64,
}

impl From<&ResponseSizeBytesMetrics> for ResponseSizeReport {
    fn from(item: &ResponseSizeBytesMetrics) -> Self {
        ResponseSizeReport {
            _title: dummy_title(),
            mean: item.mean(),
            total: item.sum(),
        }
    }
}

#[derive(Debug, Tabled, Serialize)]
#[tabled(rename_all = "Pascal")]
pub(crate) struct ErrorReport {
    #[tabled(rename = "Error")]
    #[serde(skip)]
    _title: String,
    message: String,
}

impl From<&ErrorMetrics> for Vec<ErrorReport> {
    fn from(item: &ErrorMetrics) -> Self {
        item.error_messages()
            .into_iter()
            .map(|message| ErrorReport {
                _title: dummy_title(),
                message,
            })
            .collect()
    }
}

fn dummy_title() -> String {
    " ".repeat(TITLE_PADDING_SPACES)
}

fn table<I, T>(iter: I) -> Result<String>
where
    I: IntoIterator<Item = T>,
    T: Tabled,
{
    let header_border = Border::full(' ', '─', ' ', ' ', ' ', ' ', '─', '─');
    let top_left_border = Border::full(' ', '─', ' ', '│', ' ', ' ', ' ', '┼');
    let top_bottom_border = Border::full('─', ' ', ' ', '│', ' ', '┼', ' ', ' ');

    let mut table = Table::new(iter);
    table.with(Style::blank());
    table.with(Alignment::right());
    table.modify(Rows::first(), header_border);
    table.modify(Cell::new(0, 0), top_left_border);
    table.modify(Cell::new(1, 0), top_bottom_border);

    Ok(table.to_string())
}
