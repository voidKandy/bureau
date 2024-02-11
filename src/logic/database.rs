use anyhow;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::sql::Thing;
use surrealdb::Surreal;

static DB: Lazy<Surreal<Client>> = Lazy::new(Surreal::init);

#[derive(Debug, Deserialize)]
struct Record {
    id: Thing,
}

#[derive(Debug, Deserialize)]
struct TestEntry {
    #[allow(dead_code)]
    id: Thing,
    name: String,
    description: String,
}

pub async fn connect() -> surrealdb::Result<()> {
    DB.connect::<Ws>("127.0.0.1:8000").await?;

    tracing::info!("Database connected!");

    DB.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;

    // Select a specific namespace / database
    DB.use_ns("test").use_db("test").await?;
    Ok(())
}

pub async fn test_get() -> surrealdb::Result<()> {
    let test: Vec<TestEntry> = DB.select("testentry").await?;
    tracing::info!("TEST GOT: {:?}", test);
    Ok(())
}
