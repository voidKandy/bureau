use crate::SharedState;
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
};

use super::models::{AgentView, ChatHistory, EnvView, MessageRender};
use std::time::Duration;

pub async fn env_view(
    Path(env_id): Path<String>,
    State(state): State<SharedState>,
) -> Html<String> {
    let state_read = state.write().await;
    if let Some(current_env_agents) = state_read.agent_handles_map.get(&env_id) {
        let agent_names = current_env_agents.keys().map(|k| k.to_string()).collect();

        let view = EnvView {
            id: &env_id,
            agent_names,
        };
        let render = view.render().unwrap();
        return Html(render);
    }
    let failure_string = String::from("No current env");
    Html(failure_string)
}

pub async fn agent_view(
    Path((env_id, agent_id)): Path<(String, String)>,
    // State(state): State<SharedState>,
) -> Html<String> {
    let view = AgentView {
        id: &agent_id,
        parent_id: &env_id,
    };
    Html(view.render().unwrap())
}

#[tracing::instrument(name = "Agent history", skip(state))]
pub async fn history(
    State(state): State<SharedState>,
    Path((env_id, agent_id)): Path<(String, String)>,
) -> Html<String> {
    // let mut messages = MessageVector::init();
    let state_read = state.read().await;
    tracing::info!("Got read lock on state");
    if let Some(env) = state_read.environments_map.get(&env_id) {
        tracing::info!("Got environment reference");
        // let mut notis = env.notifications.0.write().await;
        if let Ok(dispatch_read) =
            tokio::time::timeout(Duration::from_millis(200), env.dispatch.read()).await
        {
            tracing::info!("Got read lock on dispatch");
            if let Ok(agent) = dispatch_read.get_agent_ref(&agent_id) {
                tracing::info!("Got agent reference");
                let messages: Vec<MessageRender> =
                    agent.cache.as_ref().iter().map(|m| m.into()).collect();
                let history = ChatHistory { messages };
                return Html(history.render().unwrap());
            }
        }
        return Html("Couldn't get dispatch read lock".to_string());
    }

    // let template = ChatHistory { messages };
    Html(format!("{}-{}", env_id, agent_id))
}
