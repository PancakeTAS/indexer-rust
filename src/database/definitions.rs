use anyhow::Context;
use log::debug;
use surrealdb::{engine::remote::ws::Client, Surreal};

/// Initialize the database with the necessary definitions
pub async fn init(db: &Surreal<Client>) -> anyhow::Result<()> {
    // define the namespace
    debug!("Defining namespace");
    db.query("DEFINE NAMESPACE atp;")
        .await.context("Failed to define namespace atp")?;
    db.use_ns("atp").await?;

    // define the database
    debug!("Defining database");
    db.query("DEFINE DATABASE atp;")
        .await.context("Failed to define database atp")?;
    db.use_ns("atp").use_db("atp").await?;

    // TODO Add all types
    db.query("
        DEFINE TABLE did SCHEMAFULL;
        DEFINE FIELD handle ON TABLE did TYPE option<string>;
        DEFINE FIELD displayName ON TABLE did TYPE option<string>;
        DEFINE FIELD description ON TABLE did TYPE option<string>;
        DEFINE FIELD avatar ON TABLE did TYPE option<record<blob>>;
        DEFINE FIELD banner ON TABLE did TYPE option<record<blob>>;
        DEFINE FIELD labels ON TABLE did TYPE option<array>;
        DEFINE FIELD labels.* ON TABLE did TYPE string;
        DEFINE FIELD joinedViaStarterPack ON TABLE did TYPE option<record<starterpack>>;
        DEFINE FIELD pinnedPost ON TABLE did TYPE option<record<post>>;
        DEFINE FIELD createdAt ON TABLE did TYPE datetime;

        DEFINE TABLE post SCHEMAFULL;
        DEFINE FIELD text ON TABLE post TYPE string;
        ", // record<one | two>
    )
    .await?;

    Ok(())
}