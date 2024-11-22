use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Context;
use config::Args;
use ::log::error;
use tokio::runtime::Builder;
use tokio_rustls::rustls::crypto::aws_lc_rs::default_provider;

mod database;
mod websocket;
mod config;
mod log;

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
    let rt =
        if let Some(threads) = args.executors {
            Builder::new_multi_thread()
                .enable_all()
                .worker_threads(threads)
                .thread_name_fn(|| {
                    static ATOMIC: AtomicUsize = AtomicUsize::new(0);
                    let id = ATOMIC.fetch_add(1, Ordering::Relaxed);
                    format!("Tokio Async Thread {}", id)
                })
                .build().unwrap()
        } else {
            Builder::new_multi_thread()
                .enable_all()
                .build().unwrap()
        };

    // launch the application
    default_provider().install_default().unwrap();
    let err = rt.block_on(application_main(args));
    if let Err(e) = &err {  error!("{:?}", e); }

    // exit
    std::process::exit(if err.is_ok() { 0 } else { 1 });
}

/// Asynchronous main function
async fn application_main(args: Args) -> anyhow::Result<()> {

    // connect to the database
    let db = database::connect(args.dbhost, &args.username, &args.password)
        .await.context("Failed to connect to the database")?;

    // enter websocket event loop
    websocket::start(args.host, args.certificate, args.cursor, args.handlers, db)
        .await.context("WebSocket event loop failed")?;

    Ok(())
}
