use super::models::LayoutTemplate;
use crate::SharedState;
use askama::Template;
use axum::{extract::State, response::Html};

pub async fn index(State(state): State<SharedState>) -> Html<String> {
    let state_read = state.read().await;
    let environment_names = Some(
        state_read
            .environments_map
            .keys()
            .map(|k| k.to_string())
            .collect(),
    );
    let template = LayoutTemplate {
        environment_names,
        path_and_params: None,
    };
    Html(template.render().unwrap())
}
