use crate::{
    espx_env::ui_listeners::{CacheEdit, StackEdit},
    SharedState,
};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    Form,
};
use espionox::{
    agents::{memory::Message, Agent},
    language_models::LLM,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Template)]
#[template(path = "add_message_form.html")]
pub struct AddMessageForm<'a> {
    agent_id: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct AddMessage {
    role: String,
    content: String,
}

#[tracing::instrument(name = "Add message to agent", skip_all)]
pub async fn add_message(
    State(state): State<SharedState>,
    Path(agent_id): Path<String>,
    Form(add_message): Form<AddMessage>,
) -> Html<String> {
    let mut state_write = state.write().await;
    let message = Message {
        role: add_message.role.try_into().unwrap(),
        content: add_message.content,
    };
    let edit = CacheEdit {
        agent_id: agent_id.clone(),
        edit: StackEdit::PushMessageToCache { message },
    };

    match state_write.env_state.ui_handler.push_to_changes(edit) {
        Ok(_) => {
            return Html(String::from("Cache Updated!"));
        }
        Err(err) => {
            tracing::info!("Error updating cache: {:?} ", err);
            return Html(format!("Error updating cache: {:?} ", err));
        }
    }
}

pub async fn add_message_form(Path(agent_id): Path<String>) -> Html<String> {
    let form = AddMessageForm {
        agent_id: &agent_id,
    };
    Html(form.render().unwrap())
}

#[tracing::instrument(name = "Change message", skip_all)]
pub async fn message_change(
    State(state): State<SharedState>,
    Path((agent_id, idx)): Path<(String, usize)>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<String> {
    let mut state_write = state.write().await;
    if let Some(new_text) = params.get("change") {
        let edit = CacheEdit {
            agent_id,
            edit: StackEdit::EditMessageInCache {
                idx,
                new_text: new_text.to_string(),
            },
        };

        match state_write.env_state.ui_handler.push_to_changes(edit) {
            Ok(_) => {
                tracing::info!("Returning success message");

                return Html(String::from("Cache Updated!"));
            }
            Err(err) => {
                tracing::info!("Error updating cache: {:?} ", err);
                return Html(format!("Error updating cache: {:?} ", err));
            }
        }
    }

    return Html(String::from(
        "Error updating cache: no change passed in request",
    ));
}

#[tracing::instrument(name = "Delete message", skip_all)]
pub async fn message_delete(
    State(state): State<SharedState>,
    Path((agent_id, idx)): Path<(String, usize)>,
) -> Html<String> {
    let mut state_write = state.write().await;
    let edit = CacheEdit {
        agent_id,
        edit: StackEdit::RemoveMessageInCache { idx },
    };
    match state_write.env_state.ui_handler.push_to_changes(edit) {
        Ok(_) => {
            tracing::info!("Delete successful");
            return Html(String::from("Delete successful"));
        }
        Err(err) => {
            tracing::info!("Error updating cache: {:?} ", err);
            return Html(format!("Error updating cache: {:?} ", err));
        }
    }
}
