pub mod espx_env;
pub mod routing;
pub mod state;
pub mod telemetry;
pub mod view_logic;
pub mod websocket;

pub use state::*;

use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[tokio::main]
async fn main() {
    static TRACING: Lazy<()> = Lazy::new(|| {
        let default_filter_level = "info".to_string();
        let subscriber_name = "main".to_string();
        if std::env::var("MAIN_LOG").is_ok() {
            let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
            init_subscriber(subscriber);
        } else {
            let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
            init_subscriber(subscriber);
        }
    });

    Lazy::force(&TRACING);

    // let _ = database::connect().await;
    // database::test_get().await.unwrap();

    let (tx, _rx) = broadcast::channel(100);

    let state = Arc::new(RwLock::new(AppState::init(tx).await));

    let router = routing::main_router().with_state(Arc::clone(&state));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}
