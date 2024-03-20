use super::espx_env::{EnvStatesMap, EnvironmentState};
pub use super::telemetry::*;
pub use super::view_logic::*;

use axum::response::Html;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};

#[derive(Debug)]
pub struct AppState {
    pub environments_map: EnvStatesMap,
    pub tx: broadcast::Sender<Html<String>>,
}

pub type SharedState = Arc<RwLock<AppState>>;

impl AppState {
    pub async fn init(tx: broadcast::Sender<Html<String>>) -> Self {
        let mut environments_map = HashMap::new();
        let default_env = EnvironmentState::init().await.expect("Could not init env");

        environments_map.insert("Default".to_owned(), default_env);

        Self {
            environments_map,
            tx,
        }
    }
}
