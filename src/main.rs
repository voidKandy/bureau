pub mod logic;
pub mod telemetry;
use dotenv::dotenv;
pub use logic::*;
pub use telemetry::*;

use axum::{extract::State, response::Html, routing::get, Router};
use espionox::{
    environment::{
        agent::{language_models::LanguageModel, memory::*, AgentHandle},
        Environment,
    },
    Agent,
};
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{broadcast, RwLock};

#[derive(Debug)]
pub struct AppState {
    environments_map: HashMap<String, Environment>,
    agent_handles_map: HashMap<String, HashMap<String, AgentHandle>>,
    menu: bool,
    tx: broadcast::Sender<Html<String>>,
}

pub type SharedState = Arc<RwLock<AppState>>;

impl AppState {
    fn init_default_env() -> Environment {
        dotenv().ok();
        let api_key = std::env::var("OPENAI_API_KEY").unwrap();
        tracing::info!("Got openai api key: {}", api_key);
        Environment::new(Some("default"), Some(&api_key))
    }

    async fn insert_default_agents(env: &mut Environment) -> HashMap<String, AgentHandle> {
        let default_agent = Agent::new("default agent", LanguageModel::default_gpt());
        let non_default_agent = Agent::new(
            "You are the non default agent",
            LanguageModel::default_gpt(),
        );
        let handle1 = env
            .insert_agent(Some("non-default"), non_default_agent)
            .await
            .unwrap();
        let handle2 = env
            .insert_agent(Some("default"), default_agent)
            .await
            .unwrap();
        let mut map = HashMap::new();
        map.insert(handle1.id.clone(), handle1);
        map.insert(handle2.id.clone(), handle2);
        map
    }
}

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

    let _ = logic::database::connect().await;
    logic::database::test_get().await.unwrap();

    let (tx, _rx) = broadcast::channel(100);
    let mut env = AppState::init_default_env();
    let agent_handles = AppState::insert_default_agents(&mut env).await;

    let mut environments_map = HashMap::new();
    let mut agent_handles_map = HashMap::new();

    agent_handles_map.insert(env.id.clone(), agent_handles);
    environments_map.insert(env.id.clone(), env);

    let state = Arc::new(RwLock::new(AppState {
        environments_map,
        agent_handles_map,
        menu: false,
        tx,
    }));

    // let websocket_routes = routing::init_ws_routes();
    // let chat_routes = routing::init_chat_routes();

    let router = routing::main_router().with_state(Arc::clone(&state));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}
