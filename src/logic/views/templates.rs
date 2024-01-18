use super::models::{IndexTemplate, LayoutTemplate};
use crate::{AppState, SharedState};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use axum_htmx::HxCurrentUrl;
use espionox::environment::Environment;
use std::collections::HashMap;

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
