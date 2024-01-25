use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, Method},
    response::Html,
    Form,
};
use axum_htmx::HxCurrentUrl;
use espionox::environment::agent::memory::{Message, MessageMetadata, MessageRole};
use serde::Deserialize;

use crate::{views::models::MessageRender, SharedState};

#[derive(Template)]
#[template(path = "add_message_form.html")]
pub struct AddMessageForm<'a> {
    env_id: &'a str,
    agent_id: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct AddMessage {
    role: String,
    content: String,
}

pub async fn add_message(
    State(state): State<SharedState>,
    Path((env_id, agent_id)): Path<(String, String)>,
    Form(add_message): Form<AddMessage>,
) -> Html<String> {
    let mut state_write = state.write().await;
    if let Some(env) = state_write.environments_map.get_mut(&env_id) {
        if let Some(agent) = env.dispatch.write().await.get_agent_mut(&agent_id).ok() {
            let role: MessageRole = add_message.role.into();
            let message = Message {
                role,
                content: add_message.content,
                metadata: MessageMetadata::default(),
            };
            agent.cache.push(message);
        }
        return Html(String::from("Added message"));
    }
    return Html(String::from("Invalid envirnoment name in request!"));
}

pub async fn add_message_form(Path((env_id, agent_id)): Path<(String, String)>) -> Html<String> {
    let form = AddMessageForm {
        env_id: &env_id,
        agent_id: &agent_id,
    };
    Html(form.render().unwrap())
}

pub async fn message_change(
    State(state): State<SharedState>,
    Path((env_id, agent_id, role, content)): Path<(String, String, String, String)>,
    Query(params): Query<HashMap<String, String>>, // State(state): State<SharedState>,
) -> Html<String> {
    let mut state_write = state.write().await;
    if let Some(env) = state_write.environments_map.get_mut(&env_id) {
        if let Some(agent) = env.dispatch.write().await.get_agent_mut(&agent_id).ok() {
            let role =
                MessageRole::try_from(role).expect("Failed to build message role with given role");
            let origin_message = Message {
                role: role.clone(),
                content: content.clone(),
                metadata: MessageMetadata::default(),
            };
            if let Some(change) = params.get("change") {
                if change != &content {
                    let new_message = Message {
                        role,
                        content: change.to_owned(),
                        metadata: MessageMetadata::default(),
                    };

                    if let Some(message) = agent.cache.as_mut().iter_mut().find(|m| {
                        m.role == origin_message.role && m.content == origin_message.content
                    }) {
                        *message = new_message;
                        return Html(String::from("Cache Updated!"));
                    }

                    return Html(String::from("No message matching predicates!"));
                }
            } else if let Some(delete) = params.get("delete") {
                if delete == "true" {
                    if let Some(idx) = agent.cache.as_ref().iter().position(|m| {
                        m.role == origin_message.role && m.content == origin_message.content
                    }) {
                        agent.cache.as_mut().remove(idx);
                        return Html(String::from("Message Deleted!"));
                    }
                }
            }
            return Html(String::from("No change in request!"));
        }
        return Html(String::from("Invalid agent name in request!"));
    }
    return Html(String::from("Invalid envirnoment name in request!"));
}
