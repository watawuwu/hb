mod args;

use anyhow::Result;
use args::Args;
use hb::bench;
use hb::otlp::setup_metrics;
use tokio::{
    runtime::Builder,
    signal::unix::{signal, SignalKind},
};
use tokio_util::sync::CancellationToken;
use tracing::*;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse_wrapper()?;
    let req = args.request();
    let opts = args.bench_options()?;

    let mut builder = Builder::new_multi_thread();
    if let Some(threads) = args.thread {
        builder.worker_threads(threads);
    }
    let runtime = builder.enable_all().build()?;

    runtime.block_on(async move {
        let otlp_opts = args.otlp_options();
        let (provider, metrics) = setup_metrics(otlp_opts, &req).await?;

        let cxl = CancellationToken::new();
        let main_cxl: CancellationToken = cxl.child_token();

        let main_task = tokio::spawn(async move {
            let bench_cxl = main_cxl.clone();

            bench::bench(bench_cxl, req, opts, metrics).await?;
            main_cxl.cancel();
            Ok::<_, anyhow::Error>(())
        });

        tokio::spawn(async move {
            let mut sigint = signal(SignalKind::interrupt())?;
            let mut sigterm = signal(SignalKind::terminate())?;
            let mut sigquit = signal(SignalKind::quit())?;

            tokio::select! {
                _ = sigint.recv() => Ok::<_, anyhow::Error>(()),
                _ = sigterm.recv() => Ok(()),
                _ = sigquit.recv() => Ok(()),
            }?;

            info!("Signal received: stopping...");

            cxl.cancel();
            Ok::<_, anyhow::Error>(())
        });

        main_task.await??;

        provider.shutdown()?;

        Ok::<_, anyhow::Error>(())
    })?;

    Ok(())
}
