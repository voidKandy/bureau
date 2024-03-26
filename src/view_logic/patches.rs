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
    Path(agent_id): Path<String>,
    Form(add_message): Form<AddMessage>,
) -> Html<String> {
    let state_write = state.write().await;
    let message = Message {
        role: add_message.role.try_into().unwrap(),
        content: add_message.content,
    };
    let edit = CacheEdit {
        agent_id,
        edit: StackEdit::PushMessageToCache { message },
    };
    state_write
        .env_state
        .ui_handler
        .cache_changes
        .write()
        .unwrap()
        .push_back(edit);

    return Html(String::from("Cache Updated!"));

    // return add_message_form(Path((env_id, agent_id))).await;
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
    Path((_, agent_id, idx)): Path<(String, String, usize)>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    let state_write = state.write().await;
    if let Some(new_text) = params.get("change") {
        let edit = CacheEdit {
            agent_id,
            edit: StackEdit::EditMessageInCache {
                idx,
                new_text: new_text.to_string(),
            },
        };

        state_write
            .env_state
            .ui_handler
            .cache_changes
            .write()
            .unwrap()
            .push_back(edit);

        return Html(String::from("Cache Updated!"));
    }
    if let Some(delete) = params.get("delete") {
        if delete == "true" {
            let edit = CacheEdit {
                agent_id,
                edit: StackEdit::RemoveMessageInCache { idx },
            };
            state_write
                .env_state
                .ui_handler
                .cache_changes
                .write()
                .unwrap()
                .push_back(edit);
            return Html(String::from("Message Deleted!"));
        }
    }
    return Html(String::from("No sender for that agent"));
}
