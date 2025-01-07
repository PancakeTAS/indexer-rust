use std::sync::atomic::{AtomicUsize, Ordering};

use ::log::{error, info};
use anyhow::Context;
use config::Args;
use database::repo_indexer::start_full_repo_indexer;
use surrealdb::{engine::any::Any, Surreal};
use tokio::runtime::Builder;
use tokio_rustls::rustls::crypto::aws_lc_rs::default_provider;

mod config;
mod database;
mod log;
mod websocket;

/// Override the global allocator with mimalloc
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Entry point for the application
fn main() {
    // parse command line arguments
    let args = config::parse_args();

    // initialize logging and dump configuration
    log::init(args.log_level());
    args.dump();

    // build async runtime
    let rt = if let Some(threads) = args.worker_threads {
        Builder::new_multi_thread()
            .enable_all()
            .worker_threads(threads)
            .max_blocking_threads(512 * 512)
            .max_io_events_per_tick(1024 * 512)
            .thread_name_fn(|| {
                static ATOMIC: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC.fetch_add(1, Ordering::Relaxed);
                format!("Tokio Async Thread {}", id)
            })
            .build()
            .unwrap()
    } else {
        Builder::new_multi_thread()
            .enable_all()
            .max_blocking_threads(512 * 512)
            .max_io_events_per_tick(1024 * 512)
            .thread_name_fn(|| {
                static ATOMIC: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC.fetch_add(1, Ordering::Relaxed);
                format!("Tokio Async Thread {}", id)
            })
            .build()
            .unwrap()
    };

    // launch the application
    default_provider().install_default().unwrap();
    let err = rt.block_on(application_main(args));
    if let Err(e) = &err {
        error!(target: "indexer", "{:?}", e);
    }

    // exit
    std::process::exit(if err.is_ok() { 0 } else { 1 });
}

/// Asynchronous main function
async fn application_main(args: Args) -> anyhow::Result<()> {
    // connect to the database
    let db = database::connect(args.db)
        .await
        .context("Failed to connect to the database")?;

    let jetstream_hosts = vec![
        "jetstream1.us-west.bsky.network",
        "jetstream2.us-east.bsky.network",
        "test-jetstream.skyfeed.moe",
        "jetstream2.us-west.bsky.network",
        "jetstream1.us-east.bsky.network",
    ];

    for host in jetstream_hosts {
        let db_clone = db.clone();
        let certificate = args.certificate.clone();

        std::thread::Builder::new()
            .name(format!("Jetstream Consumer {}", host))
            .spawn(move || {
                Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .unwrap()
                    .block_on(async {
                        start_jetstream_consumer(db_clone, host.to_string(), certificate)
                            .await
                            .context("jetstream consumer failed")
                            .unwrap();
                    });
            })
            .context("Failed to spawn jetstream consumer thread")?;
    }

    let db_clone = db.clone();
    tokio::spawn(async move {
        let mut last_count = 0;
        loop {
            let mut res = db_clone
                .query("SELECT count() as c FROM li_did GROUP ALL;")
                .await
                .unwrap();
            let count: Option<i64> = res.take((0, "c")).unwrap();
            info!(
                "fully indexed repo count: {} with {} repos/10s",
                count.unwrap_or(0),
                count.unwrap_or(0) - last_count
            );
            last_count = count.unwrap_or(0);
            tokio::time::sleep(std::time::Duration::from_millis(10000)).await;
        }
    });

    if args.mode == "full" {
        start_full_repo_indexer(db, args.max_concurrent_requests.unwrap_or(num_cpus::get() * 50)).await?;
    }

    loop {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}
async fn start_jetstream_consumer(
    db: Surreal<Any>,
    host: String,
    certificate: String,
) -> anyhow::Result<()> {
    // fetch initial cursor
    let cursor = database::fetch_cursor(&db, &host)
        .await
        .context("Failed to fetch cursor from database")?
        .map_or(0, |e| e.time_us);

    // enter websocket event loop
    websocket::start(host, certificate, cursor, db)
        .await
        .context("WebSocket event loop failed")?;

    Ok(())
}
