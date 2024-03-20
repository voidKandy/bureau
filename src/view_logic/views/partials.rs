use crate::SharedState;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
};

use super::models::{AgentView, ChatHistory, EnvView, MessageRender};

pub async fn env_view(
    Path(env_id): Path<String>,
    State(state): State<SharedState>,
) -> Html<String> {
    let state_read = state.write().await;
    if let Some(agent_names) = state_read
        .environments_map
        .get(&env_id)
        .and_then(|env| Some(env.agent_names()))
    {
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
        agent_id: &agent_id,
        env_id: &env_id,
    };
    Html(view.render().unwrap())
}

#[tracing::instrument(name = "Agent history", skip(state))]
pub async fn history(
    State(state): State<SharedState>,
    Path((env_id, agent_id)): Path<(String, String)>,
) -> Html<String> {
    let mut state_read = state.write().await;
    tracing::info!("Got read lock on state");
    if let Some(env) = state_read.environments_map.get_mut(&env_id) {
        tracing::info!("Got environment reference");
        env.ui_channel.update_cache_states().await;
        if let Some(cache) = env.ui_channel.get_agent_cache(&agent_id) {
            tracing::info!("Got agent reference");
            let messages: Vec<MessageRender> = cache.as_ref().iter().map(|m| m.into()).collect();
            let history = ChatHistory {
                env_id,
                agent_id,
                messages,
            };
            return Html(history.render().unwrap());
        }
    }

    Html(format!("{}-{}", env_id, agent_id))
}
