use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    Form,
};
use espionox::agents::memory::Message;
use serde::Deserialize;

use crate::{
    espx_env::ui_listeners::{CacheEdit, StackEdit},
    SharedState,
};
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
        if let Some(sender) = env.ui_channel.get_agent_edit_sender(&agent_id) {
            let message = Message {
                role: add_message.role.into(),
                content: add_message.content,
            };
            let edit = StackEdit::PushMessageToCache { message };
            sender.send(CacheEdit { agent_id, edit }).await.unwrap();

            return Html(String::from("Cache Updated!"));
        }
        return add_message_form(Path((env_id, agent_id))).await;
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
    Path((env_id, agent_id, idx)): Path<(String, String, usize)>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    let mut state_write = state.write().await;
    if let Some(env) = state_write.environments_map.get_mut(&env_id) {
        if let Some(sender) = env.ui_channel.get_agent_edit_sender(&agent_id) {
            if let Some(new_text) = params.get("change") {
                let edit = StackEdit::EditMessageInCache {
                    idx,
                    new_text: new_text.to_string(),
                };
                sender.send(CacheEdit { agent_id, edit }).await.unwrap();

                return Html(String::from("Cache Updated!"));
            }
            if let Some(delete) = params.get("delete") {
                if delete == "true" {
                    let edit = StackEdit::RemoveMessageInCache { idx };
                    sender.send(CacheEdit { agent_id, edit }).await.unwrap();
                    return Html(String::from("Message Deleted!"));
                }
            }
        }
        return Html(String::from("No sender for that agent"));
    }
    return Html(String::from("No env"));
}
