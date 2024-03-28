use crate::SharedState;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
};

use super::models::{AgentView, ChatHistory, MessageRender};

pub async fn agent_view(
    Path(agent_id): Path<String>,
    // State(state): State<SharedState>,
) -> Html<String> {
    let view = AgentView {
        agent_id: &agent_id,
    };
    Html(view.render().unwrap())
}

#[tracing::instrument(name = "Agent history", skip(state))]
pub async fn history(
    State(state): State<SharedState>,
    Path(agent_id): Path<String>,
) -> Html<String> {
    let state_read = state.read().await;
    let caches = &state_read.env_state.ui_handler;
    if let Some(cache) = caches.get_state_of_agent(&agent_id) {
        tracing::info!("Got agent reference");
        let messages: Vec<MessageRender> = cache.as_ref().iter().map(|m| m.into()).collect();
        let history = ChatHistory { agent_id, messages };
        return Html(history.render().unwrap());
    }

    Html(format!("{}", agent_id))
}
