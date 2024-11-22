use std::{sync::{atomic::{AtomicU64, Ordering}, mpsc::{self, Receiver, Sender}, Arc, Mutex}, thread, time::Duration};

use anyhow::Context;
use fastwebsockets::{OpCode, WebSocket};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use log::{debug, error, info, trace, warn};
use surrealdb::{engine::remote::ws::Client, Surreal};
use tokio::time::sleep;

mod conn;
mod handler;
mod events;

/// Shared state for the websocket module
#[derive(Debug)]
struct SharedState {
    rx: Arc<Mutex<Receiver<String>>>,
    db: Surreal<Client>,
    cursor: AtomicU64
}

impl SharedState {
    /// Update the cursor
    pub fn update_cursor(&self, cursor: u64) {
        self.cursor.store(cursor, Ordering::Relaxed);
    }
}

/// Subscribe to a websocket server
pub async fn start(
    host: String, certificate: String, cursor: Option<u64>, handlers: Option<usize>, db: Surreal<Client>)
    -> anyhow::Result<()> {

    // create a shared state
    info!(target: "indexer", "Entering websocket loop");
    let (tx, rx) = mpsc::channel();
    let state = Arc::new(SharedState {
        rx: Arc::new(Mutex::new(rx)),
        db,
        cursor: AtomicU64::new(cursor.unwrap_or(0))
    });

    // spin up the thread pool
    let cpus = handlers.unwrap_or_else(num_cpus::get);
    info!(target: "indexer", "Spinning up {} handler threads", cpus);
    for i in 0..cpus {
        let state = state.clone();
        thread::Builder::new()
            .name(format!("WebSocket Handler Thread {}", i))
            .spawn(move || {
                let res = thread_handler(state);
                if let Err(e) = res {
                    error!("Handler thread {} failed: {:?}", i, e);
                } else {
                    debug!("Handler thread {} exited", i);
                }
            })
            .context("Failed to spawn handler thread")?;
    };

    // loop infinitely, ensuring connection aborts are handled
    loop {
        // get current cursor
        let cursor = {
            let c = (&state).cursor.load(Ordering::Relaxed) as u64;
            if c == 0 { None } else { Some(c) }
        };

        // create websocket connection
        info!(target: "indexer", "Establishing new connection to: {}", host);
        let ws = conn::connect_tls(&host, &certificate, cursor).await;
        if let Err(e) = ws {
            warn!(target: "indexer", "Unable to open websocket connection to {}: {:?}", host, e);
            sleep(Duration::from_secs(5)).await;
            continue;
        }
        let ws = ws.unwrap();

        // handle the websocket connection
        let res = manage_ws(&tx, ws).await;
        if let Err(e) = res {
            warn!(target: "indexer", "Websocket connection failed: {:?}", e);
        }

        // rewind cursor by 2 seconds
        {
            let two_seconds = 2 * 1_000_000; // 2 seconds in microseconds
            let cursor = (&state).cursor.fetch_sub(two_seconds, Ordering::Relaxed);
            info!(target: "indexer", "Rewinding cursor by 2 seconds: {} -> {}", cursor, cursor - two_seconds);
        }

        // let the server breathe
        sleep(Duration::from_millis(200)).await;
    }
}

async fn manage_ws(tx: &Sender<String>, mut ws: WebSocket<TokioIo<Upgraded>>) -> anyhow::Result<()> {
    info!(target: "indexer", "Handling websocket connection");
    loop {
        // try to read a message
        let msg = ws.read_frame()
            .await.context("Failed to read frame from websocket")?;

        // handle message
        match msg.opcode {
            // spec states only text frames are allowed
            OpCode::Continuation | OpCode::Binary | OpCode::Ping | OpCode::Pong => {
                warn!(target: "indexer", "Unexpected opcode received: {:?}", msg.opcode);
            },
            // can be emitted by the server
            OpCode::Close => {
                anyhow::bail!("Unexpected connection close received: {:?}", msg.payload);
            },
            // handle text message
            OpCode::Text => {
                trace!(target: "indexer", "Received text message: {}", msg.payload.len());
                let text = String::from_utf8(msg.payload.to_vec())
                    .context("Failed to decode text message")?;
                tx.send(text)
                    .context("Failed to send message to handler thread")?;
            }
        };
    }
}

fn thread_handler(state: Arc<SharedState>)
    -> anyhow::Result<()> {
    // loop infinitely, handling messages
    loop {
        // get the next message
        let msg = state.rx.lock().unwrap().recv();
        if let Err(e) = msg {
            debug!(target: "indexer", "Receiver closed: {:?}", e);
            break;
        }
        let msg = msg.unwrap();

        // handle the message
        trace!(target: "indexer", "Handling message: {}", &msg);
        let res = handler::handle_message(&state, msg);
        if let Err(e) = res {
            warn!(target: "indexer", "Failed to handle message: {:?}", e);
        }
    }

    Ok(())
}
