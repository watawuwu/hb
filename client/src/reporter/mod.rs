use formatter::{format_duration, format_si};
use std::time::Duration;

mod components;
pub mod exporter;
pub mod formatter;
mod metrics;

pub const REPORT_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub enum ExecMode {
    ByCount(u64),
    ByDuration(Duration),
}

impl ExecMode {
    pub(crate) fn state_progress(&self, elapsed: u64, count: u64) -> (u64, String, u64, String) {
        match self {
            ExecMode::ByCount(goal) => (count, format_si(&count), *goal, format_si(goal)),
            ExecMode::ByDuration(goal) => {
                let elapsed = elapsed.min(goal.as_secs());
                (
                    elapsed,
                    format_duration(&elapsed),
                    goal.as_secs(),
                    format_duration(&goal.as_secs()),
                )
            }
        }
    }
}
