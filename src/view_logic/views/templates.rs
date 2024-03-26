use super::models::LayoutTemplate;
use crate::SharedState;
use askama::Template;
use axum::{extract::State, response::Html};

pub async fn index(State(state): State<SharedState>) -> Html<String> {
    let state_read = state.read().await;
    let agent_names = Some(state_read.env_state.agent_names());
    let template = LayoutTemplate {
        agent_names,
        path_and_params: None,
    };
    Html(template.render().unwrap())
}
