use anyhow::Context;
use log::{debug, info};
use surrealdb::{engine::remote::ws::{Client, Ws}, opt::auth::Root, Surreal};

mod definitions;

/// Connect to the database
pub async fn connect(dbhost: String, username: &str, password: &str)
    -> anyhow::Result<Surreal<Client>> {
    // connect to the database
    info!("Connecting to the database at {}", dbhost);
    let db: Surreal<Client> = Surreal::init();
    db.connect::<Ws>(&dbhost)
        .await.with_context(|| format!("Unable to open database connection to {}", dbhost))?;

    // sign in to the server
    debug!("Signing in as {}", username);
    db.signin(Root { username, password })
        .await.with_context(|| format!("Failed to sign in as {}", username))?;

    definitions::init(&db)
        .await.context("Failed to initialize database schema")?;

    Ok(db)
}
