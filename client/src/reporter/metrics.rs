use crate::stats::percentile;
use crate::time::now_ts;
use anyhow::{Context, Result};
use opentelemetry_sdk::metrics::data::{
    self, GaugeDataPoint, Histogram, HistogramDataPoint, ResourceMetrics, Sum, SumDataPoint,
};
use std::collections::BTreeMap;
use std::ops::AddAssign;

#[derive(Debug, Default)]
struct Bucket {
    count: u64,
    bounds: Vec<f64>,
    bucket_counts: Vec<u64>,
}

impl Bucket {
    fn new(count: u64, bounds: Vec<f64>, bucket_counts: Vec<u64>) -> Self {
        Self {
            count,
            bounds,
            bucket_counts,
        }
    }

    fn count(&self) -> u64 {
        self.count
    }
}

impl AddAssign for Bucket {
    fn add_assign(&mut self, rhs: Self) {
        self.count += rhs.count;
        self.bounds = rhs.bounds;
        self.bucket_counts = rhs.bucket_counts;
    }
}
#[derive(Debug, Default)]
pub struct RequestDurationSecondsMetrics {
    data_points: Vec<HistogramDataPoint<f64>>,
    counts: BTreeMap<String, Bucket>,
    last_count: u64,
    last_ts: f64,
}

impl RequestDurationSecondsMetrics {
    const NAME: &'static str = "http_client_request_duration";

    pub(crate) fn try_find(
        meter: &ResourceMetrics,
        last_count: u64,
        last_ts: f64,
    ) -> Result<Option<Self>> {
        let Some(hist) = find_metrcis::<Histogram<f64>>(meter, Self::NAME) else {
            return Ok(None);
        };

        let mut counts = BTreeMap::new();
        for point in &hist.data_points {
            let status = Self::status(point)?;
            let status_key = Self::status_key(&status)?;

            let status = Bucket::new(
                point.count,
                point.bounds.clone(),
                point.bucket_counts.clone(),
            );

            *counts.entry(status_key.to_string()).or_default() += status;
        }

        Ok(Some(Self {
            data_points: hist.data_points.clone(),
            counts,
            last_count,
            last_ts,
        }))
    }

    fn status(point: &HistogramDataPoint<f64>) -> Result<String> {
        let status = point
            .attributes
            .iter()
            .find(|keyval| keyval.key.as_str() == "status")
            .context("status keyval not found")?
            .value
            .as_str()
            .to_string();

        Ok(status)
    }

    fn status_key(status: &str) -> Result<&'static str> {
        match status.parse::<u16>()? {
            200..=299 => Ok("2xx"),
            300..=399 => Ok("3xx"),
            400..=499 => Ok("4xx"),
            500..=599 => Ok("5xx"),
            _ => Ok("999"),
        }
    }

    fn elapsed_secs(&self) -> f64 {
        now_ts() - self.last_ts
    }

    fn sum(&self) -> f64 {
        self.data_points.iter().map(|point| point.sum).sum()
    }

    // for SummaryReport
    // =================================================================================
    pub(crate) fn http_success_rate(&self) -> f64 {
        let total = self.count();

        if total == 0 {
            return 0.0;
        }

        let _200_cnt = self.status_2xx_count();
        let _300_cnt = self.status_3xx_count();
        let succ_cnt = _200_cnt + _300_cnt;

        succ_cnt as f64 / total as f64 * 100.0
    }

    pub(crate) fn rps(&self) -> u64 {
        let elapsed_secs = self.elapsed_secs();

        if elapsed_secs == 0.0 {
            return 0;
        }

        let inc = self.count() - self.last_count;
        (inc as f64 / elapsed_secs) as u64
    }

    // for CounterReport
    // =================================================================================
    pub(crate) fn status_2xx_count(&self) -> u64 {
        self.counts
            .get("2xx")
            .map(|bucket| bucket.count())
            .unwrap_or_default()
    }

    pub(crate) fn status_3xx_count(&self) -> u64 {
        self.counts
            .get("3xx")
            .map(|bucket| bucket.count())
            .unwrap_or_default()
    }

    pub(crate) fn status_4xx_count(&self) -> u64 {
        self.counts
            .get("4xx")
            .map(|bucket| bucket.count())
            .unwrap_or_default()
    }

    pub(crate) fn status_5xx_count(&self) -> u64 {
        self.counts
            .get("5xx")
            .map(|bucket| bucket.count())
            .unwrap_or_default()
    }

    pub(crate) fn count(&self) -> u64 {
        self.counts.values().map(|bucket| bucket.count()).sum()
    }

    // for DurationReport
    // =================================================================================
    pub(crate) fn mean(&self) -> f64 {
        let cnt = self.count();
        if cnt == 0 {
            return 0.0;
        }
        self.sum() / cnt as f64
    }

    fn percentile(&self, percent: f64) -> f64 {
        self.counts.get("2xx").map_or(0.0, |bucket| {
            let pt = percentile(percent, &bucket.bucket_counts, &bucket.bounds).unwrap_or(0.0);
            self.max().min(pt)
        })
    }

    pub(crate) fn p50(&self) -> f64 {
        self.percentile(0.5)
    }

    pub(crate) fn p95(&self) -> f64 {
        self.percentile(0.95)
    }

    pub(crate) fn p99(&self) -> f64 {
        self.percentile(0.99)
    }

    pub(crate) fn min(&self) -> f64 {
        self.data_points
            .iter()
            .map(|point| point.min.unwrap_or_default())
            .reduce(f64::min)
            .unwrap_or(0.0)
    }

    pub(crate) fn max(&self) -> f64 {
        self.data_points
            .iter()
            .map(|point| point.max.unwrap_or_default())
            .reduce(f64::max)
            .unwrap_or(0.0)
    }
}
pub struct ResponseSizeBytesMetrics {
    data_points: Vec<HistogramDataPoint<u64>>,
}

impl ResponseSizeBytesMetrics {
    const NAME: &'static str = "http_client_response_size";

    pub(crate) fn find(meter: &ResourceMetrics) -> Option<Self> {
        let hist = find_metrcis::<Histogram<u64>>(meter, Self::NAME)?;

        Some(ResponseSizeBytesMetrics {
            data_points: hist.data_points.clone(),
        })
    }

    pub(crate) fn sum(&self) -> u64 {
        self.data_points.iter().map(|point| point.sum).sum()
    }

    pub(crate) fn count(&self) -> u64 {
        self.data_points.iter().map(|point| point.count).sum()
    }

    pub(crate) fn mean(&self) -> u64 {
        let cnt = self.count();
        if cnt > 0 { self.sum() / cnt } else { 0 }
    }
}

pub struct ErrorMetrics {
    //error_counts: BTreeMap<String, usize>,
    sum: Vec<SumDataPoint<u64>>,
}

impl ErrorMetrics {
    const NAME: &'static str = "http_client_errors";

    pub(crate) fn find(meter: &ResourceMetrics) -> Option<Self> {
        let hist = find_metrcis::<Sum<u64>>(meter, Self::NAME)?;
        Some(ErrorMetrics {
            sum: hist.data_points.clone(),
        })
    }

    pub(crate) fn error_messages(&self) -> Vec<String> {
        self.sum
            .iter()
            .filter_map(|point| {
                point
                    .attributes
                    .iter()
                    .find(|keyval| keyval.key.as_str() == "phase")
                    .map(|keyval| {
                        format!("phase: {}, count: {}", keyval.value.as_str(), point.value)
                    })
            })
            .collect()
    }
}

pub struct StartTimeMetrics {
    data_points: Vec<GaugeDataPoint<f64>>,
}

impl StartTimeMetrics {
    const NAME: &'static str = "http_client_start_time";

    pub(crate) fn find(meter: &ResourceMetrics) -> Option<Self> {
        let gauge = find_metrcis::<data::Gauge<f64>>(meter, Self::NAME)?;
        Some(StartTimeMetrics {
            data_points: gauge.data_points.clone(),
        })
    }

    pub(crate) fn elapsed_time(&self) -> u64 {
        let delta = now_ts() - self.start_time();
        delta.round() as u64
    }

    fn start_time(&self) -> f64 {
        self.data_points
            .iter()
            .map(|point| point.value)
            .next()
            .unwrap_or(0.0)
    }
}

fn find_metrcis<'a, T: 'static>(meter: &'a ResourceMetrics, name: &'a str) -> Option<&'a T> {
    let metric = meter
        .scope_metrics
        .iter()
        .flat_map(|scope_meter| &scope_meter.metrics)
        .find(|metric| metric.name == name)?;

    metric.data.as_any().downcast_ref::<T>()
}
