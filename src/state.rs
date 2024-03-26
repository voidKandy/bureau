use super::espx_env::EnvironmentState;
pub use super::telemetry::*;
pub use super::view_logic::*;

use axum::response::Html;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug)]
pub struct AppState {
    pub env_state: EnvironmentState,
    pub tx: broadcast::Sender<Html<String>>,
}

pub type SharedState = Arc<RwLock<AppState>>;

impl AppState {
    pub async fn init(tx: broadcast::Sender<Html<String>>) -> Self {
        let env_state = EnvironmentState::init().await.expect("Could not init env");
        Self { env_state, tx }
    }
}
