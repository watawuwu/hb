use crate::http::{Client, Request};
use crate::otlp::Metrics;
use anyhow::{Result, bail};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Barrier, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::*;

type BoxedFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
type BoxedAsyncClosure = Box<dyn Fn() -> BoxedFuture + Send>;

pub async fn bench(
    cxl: CancellationToken,
    req: Request,
    opts: BenchOption,
    meter: Metrics,
) -> Result<()> {
    let clients = opts.clients;
    let iter = Arc::new(Mutex::new(opts.iter()));
    let barrier = Arc::new(Barrier::new(clients));

    let mut handles = Vec::new();
    for _ in 0..clients {
        let iter = Arc::clone(&iter);
        let cxl = cxl.clone();
        let cli = Client::try_new(req.clone())?;
        let meter = meter.clone();
        let bucket = opts.token_bucket();
        let barrier = barrier.clone();

        let handle = tokio::spawn(async move {
            let task = create_request_task(cli, meter.clone());

            let count = iter.lock().await.next();
            let duration = opts.duration;

            barrier.wait().await;

            meter.record_start_time();

            match (count, duration, bucket) {
                // Count specified
                (Some(c), None, None) => run_until_count(task, &cxl, c).await,

                // Duration specified
                (None, Some(d), None) => run_until_duration(task, &cxl, d).await,

                // Count & Bucket specified
                (Some(c), None, Some(Ok(b))) => run_until_count_with_bucket(task, &cxl, c, b).await,

                // Duration & Bucket specified
                (None, Some(d), Some(Ok(b))) => {
                    run_until_duration_with_bucket(task, &cxl, d, b).await
                }

                // Either num or duration must be specified
                _ => bail!("Either num or duration must be specified."),
            }?;
            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}

fn create_request_task(cli: Client, meter: Metrics) -> BoxedAsyncClosure {
    Box::new(move || {
        let cli = cli.clone();
        let meter = meter.clone();

        Box::pin(async move {
            let result = request(cli.clone(), meter.clone()).await;
            if let Err(err) = result {
                debug!("error: {:?}", err);
            }
            Ok(())
        })
    })
}

async fn run_until_count<F, Fut, T>(f: F, cxl: &CancellationToken, count: u64) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    for _ in 0..count {
        if cxl.is_cancelled() {
            break;
        }
        f().await?;
    }
    Ok(())
}

async fn run_until_count_with_bucket<F, Fut, T>(
    f: F,
    cxl: &CancellationToken,
    count: u64,
    mut bucket: TokenBucket,
) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    for _ in 0..count {
        if cxl.is_cancelled() {
            break;
        }
        bucket.try_acquire().await;
        f().await?;
    }
    Ok(())
}

async fn run_until_duration<F, Fut, T>(
    f: F,
    cxl: &CancellationToken,
    duration: Duration,
) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let start = Instant::now();
    while start.elapsed() < duration && !cxl.is_cancelled() {
        f().await?;
    }
    Ok(())
}

async fn run_until_duration_with_bucket<F, Fut, T>(
    f: F,
    cxl: &CancellationToken,
    duration: Duration,
    mut bucket: TokenBucket,
) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let start = Instant::now();
    bucket.update();
    while start.elapsed() < duration && !cxl.is_cancelled() {
        bucket.try_acquire().await;
        f().await?;
    }
    Ok(())
}

async fn request(client: Client, meter: Metrics) -> Result<()> {
    let start = Instant::now();
    let result = client.request().await;

    let (status, size) = match result {
        Ok((status, size)) => (status, size),
        Err(err) => {
            let phase = match err.downcast_ref::<reqwest::Error>() {
                Some(err) if err.is_timeout() => "timeout",
                Some(err) if err.is_connect() => "connect",
                Some(err) if err.is_request() => "request",
                Some(err) if err.is_redirect() => "redirect",
                _ => "unknown",
            };

            meter.record_error(phase);
            return Err(err);
        }
    };
    let elapsed = start.elapsed().as_secs_f64();

    meter.record_duration(elapsed, status);
    meter.record_response_size(size);

    Ok(())
}

struct TokenBucket {
    capacity: f64, // Maximum number of tokens in the bucket. Uses same value as fill_rate but can be adjusted to allow bursts
    tokens: f64,   // Current number of tokens
    last_refill: Instant, // Time when tokens were last refilled
    fill_rate: f64, // Rate of token addition per second (same as capacity)
}

impl TokenBucket {
    /// rps: Maximum number of requests per second
    pub fn try_new(rps: u64, clients: usize) -> Result<Self> {
        if clients == 0 {
            bail!("Number of clients must be greater than zero");
        }
        if rps < clients as u64 {
            bail!("RPS must be greater than or equal to the number of clients");
        }
        if rps == 0 {
            bail!("RPS must be greater than zero");
        }

        let capacity = (rps / clients as u64) as f64;
        let fill_rate = (rps / clients as u64) as f64;
        Ok(Self {
            capacity,
            tokens: 0.0,
            last_refill: Instant::now(),
            fill_rate, // Considered the same as RPS
        })
    }

    pub fn update(&mut self) {
        self.last_refill = Instant::now();
    }

    /// Acquire tokens for one request
    /// If tokens are insufficient, wait until the required amount is accumulated
    pub async fn try_acquire(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        // Refill tokens according to elapsed time
        let new_tokens = elapsed * self.fill_rate;
        if new_tokens > 0.0 {
            self.tokens = (self.tokens + new_tokens).min(self.capacity);
            self.last_refill = now;
        }

        if self.tokens > 0.0 {
            self.tokens -= 1.0;
        } else {
            let wait_time = Duration::from_secs_f64(1.0 / self.fill_rate);
            tokio::time::sleep(wait_time).await;
            Box::pin(self.try_acquire()).await;
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchOption {
    pub count: Option<u64>,
    pub duration: Option<Duration>,
    pub clients: usize,
    pub rps: Option<u64>,
}

impl BenchOption {
    pub fn try_new(
        count: Option<u64>,
        duration: Option<Duration>,
        clients: usize,
        rps: Option<u64>,
    ) -> Result<Self> {
        if clients == 0 {
            bail!("Number of clients must be greater than zero");
        }

        if count.is_some() && duration.is_some() {
            bail!("Either count or duration must be specified");
        }

        if count.is_none() && duration.is_none() {
            bail!("Either count or duration must be specified");
        }

        if let Some(count) = count {
            if clients > count as usize {
                bail!("Number of clients must be greater than or equal to the number of requests");
            }
        }

        Ok(Self {
            count,
            duration,
            clients,
            rps,
        })
    }

    fn token_bucket(&self) -> Option<Result<TokenBucket>> {
        self.rps.map(|rps| TokenBucket::try_new(rps, self.clients))
    }

    pub fn iter(&self) -> BenchOptionIterator {
        let count = self.count.unwrap_or(0);
        let clients_left = if count == 0 { 0 } else { self.clients };
        let chunk_size = count / self.clients as u64;

        BenchOptionIterator {
            remaining: count,
            chunk_size,
            clients_left,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BenchOptionIterator {
    remaining: u64,
    chunk_size: u64,
    clients_left: usize,
}

// Simplified implementation of BenchOptionIterator
impl Iterator for BenchOptionIterator {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.clients_left == 0 {
            return None;
        }

        self.clients_left -= 1;
        let next = if self.clients_left == 0 {
            std::mem::replace(&mut self.remaining, 0)
        } else {
            let chunk = self.chunk_size;
            self.remaining -= chunk;
            chunk
        };
        debug!("BenchOptionIterator, next: {}", next);
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bench_option_iterator_exact_division() {
        let bench_option = BenchOption::try_new(Some(9), None, 3, None).unwrap();

        let mut iter = bench_option.iter();

        // When the number of requests is exactly divisible by the number of clients
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_bench_option_iterator() {
        let bench_option = BenchOption::try_new(Some(10), None, 3, None).unwrap();

        let mut iter = bench_option.iter();

        // Distribute 10 requests among 3 clients
        assert_eq!(iter.next(), Some(3)); // First client
        assert_eq!(iter.next(), Some(3)); // Second client
        assert_eq!(iter.next(), Some(4)); // Last client gets the remainder
        assert_eq!(iter.next(), None); // No more requests
    }

    #[test]
    fn test_bench_option_iterator_single_client() {
        let bench_option = BenchOption::try_new(Some(5), None, 1, None).unwrap();

        let mut iter = bench_option.iter();

        // Assign all 5 requests to a single client
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_bench_option_iterator_no_requests() {
        let bench_option = BenchOption::try_new(None, None, 3, None);
        assert!(bench_option.is_err());
    }

    #[test]
    fn test_bench_option_iterator_zero_clients() {
        let bench_option = BenchOption::try_new(Some(5), None, 0, None);
        assert!(bench_option.is_err());
    }

    #[test]
    fn test_bench_option_iterator_more_clients_than_requests() {
        let bench_option = BenchOption::try_new(Some(2), None, 5, None);
        assert!(bench_option.is_err());
    }

    #[test]
    fn test_token_bucket_initialization_various_rates() {
        // Standard case - 100 RPS
        let bucket = TokenBucket::try_new(100, 1).unwrap();
        assert_eq!(bucket.capacity, 100.0);
        assert_eq!(bucket.tokens, 0.0);

        // Low RPS case
        let bucket = TokenBucket::try_new(5, 1).unwrap();
        assert_eq!(bucket.capacity, 5.0);
        assert_eq!(bucket.tokens, 0.0);

        // High RPS case
        let bucket = TokenBucket::try_new(1000, 1).unwrap();
        assert_eq!(bucket.capacity, 1000.0);
        assert_eq!(bucket.tokens, 0.0);

        // Multiple clients case
        let bucket = TokenBucket::try_new(100, 4).unwrap();
        assert_eq!(bucket.capacity, 25.0); // (100 RPS / 4 clients) / (1000ms / 100ms window)
        assert_eq!(bucket.tokens, 0.0);
    }

    #[test]
    fn test_token_bucket_error_cases_extended() {
        // When number of clients is zero
        assert!(TokenBucket::try_new(100, 0).is_err());

        // When RPS is less than number of clients
        assert!(TokenBucket::try_new(5, 10).is_err());
        assert!(TokenBucket::try_new(1, 2).is_err());
        assert!(TokenBucket::try_new(99, 100).is_err());

        // Edge cases that should work
        assert!(TokenBucket::try_new(100, 100).is_ok());
        assert!(TokenBucket::try_new(1, 1).is_ok());
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::try_new(10, 1).unwrap();
        bucket.update();

        // Wait for 0.1 seconds - should accumulate 1 tokens
        tokio::time::sleep(Duration::from_millis(100)).await;
        bucket.try_acquire().await;
        assert!(bucket.tokens >= 0.0 && bucket.tokens <= 0.1); // Should have ~1 tokens left after consuming one

        // Consume all remaining tokens
        for _ in 0..1 {
            bucket.try_acquire().await;
        }
        assert!(bucket.tokens < 0.1); // Should be almost 0
    }
}
