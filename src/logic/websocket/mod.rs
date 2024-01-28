pub mod models;
use crate::SharedState;
use askama::Template;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::{Html, IntoResponse},
};
use espionox::environment::{
    agent::{
        language_models::openai::gpt::streaming_utils::CompletionStreamStatus,
        memory::Message as EspxMessage,
    },
    dispatch::ThreadSafeStreamCompletionHandler,
};
use futures::{sink::SinkExt, stream::StreamExt};
use markdown::to_html;

use self::models::UserMessage;

#[derive(Debug, Clone, PartialEq)]
enum WsRequest {
    PromptAgent { user_input: String },
    // NewChat { chat_name: String, agent: Agent },
    Empty,
}

impl TryFrom<serde_json::Value> for WsRequest {
    type Error = anyhow::Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value.get("user_input") {
            Some(user_input) => Ok(WsRequest::PromptAgent {
                user_input: user_input.to_string().replace('"', ""),
            }),
            None => Err(anyhow::anyhow!("No user input field")),
        }
    }
}

#[derive(Debug)]
enum WsHxTrigger<'t> {
    UserInput { agent_id: &'t str, env_id: &'t str },
    Other,
}

impl<'t> WsHxTrigger<'t> {
    fn try_from_trigger_and_name(hx_trigger: &'t str, hx_trigger_name: &'t str) -> Option<Self> {
        match hx_trigger {
            "user-input-form" => {
                if let Some(env_and_agent_id) = hx_trigger_name.strip_suffix("-agent-form") {
                    if let Some((env_id, agent_id)) = env_and_agent_id.split_once('-') {
                        return Some(Self::UserInput { agent_id, env_id });
                    }
                }
                None
            }
            _ => Some(Self::Other),
        }
    }
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

// This function deals with a single websocket connection, i.e., a single
// connected client / user, for which we will spawn two independent tasks (for
// receiving / sending chat messages).
#[tracing::instrument(name = "Main websocket function", skip(stream, state))]
async fn websocket(stream: WebSocket, state: SharedState) {
    tracing::info!("Websocket opened");
    // By splitting, we can send and receive at the same time.
    let (mut sender, mut receiver) = stream.split();

    // Username gets set in the receive loop, if it's valid.
    // Loop until a text message is found.

    let mut rx = state.write().await.tx.subscribe();

    // Spawn the first task that will receive broadcast messages and send text
    // messages over the websocket to our client.
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // In any websocket error, break loop.
            if sender
                .send(Message::Text(format!("{:?}", msg)))
                .await
                .is_err()
            {
                tracing::error!("Error in websocket");
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            tracing::info!("Received message: {:?}", text);
            let mut state_write = state.write().await;
            let tx = state_write.tx.clone();
            // let current_env = state_write.environments_map.get_mut(&env_id).unwrap();

            let mut json_rq: serde_json::Value = serde_json::from_str(&text).unwrap();

            let headers = json_rq.get("HEADERS").expect("Failed to get headers");
            let hx_trigger = headers
                .get("HX-Trigger")
                .unwrap()
                .to_string()
                .replace('"', "");
            let hx_trigger_name = headers
                .get("HX-Trigger-Name")
                .unwrap()
                .to_string()
                .replace('"', "");

            let ws_hx_trigger =
                WsHxTrigger::try_from_trigger_and_name(&hx_trigger, &hx_trigger_name);

            if let Some(obj) = json_rq.as_object_mut() {
                obj.remove("HEADERS");
            }
            let ws_message = WsRequest::try_from(json_rq).expect("Failed to get message from json");
            tracing::info!(
                "Request deserialized: {:?}\n With trigger: {:?}",
                ws_message,
                ws_hx_trigger
            );
            let user_message_option: Option<UserMessage> = (&ws_message).try_into().ok();
            if let Some(msg) = user_message_option {
                tracing::info!(
                    "Sending user message back to client: {}",
                    msg.render().unwrap()
                );
                // Should also send an indicator in the assistant-message
                let _ = tx.send(Html(msg.render().unwrap()));
            }

            match ws_message {
                WsRequest::PromptAgent { user_input } => {
                    if let Some((env_id, agent_id)) = match ws_hx_trigger {
                        Some(WsHxTrigger::UserInput { agent_id, env_id }) => {
                            Some((env_id, agent_id))
                        }
                        _ => None,
                    } {
                        let current_env = state_write.environments_map.get_mut(env_id).unwrap();
                        if !current_env.is_running() {
                            let _ = current_env.spawn().await.unwrap();
                        }
                        let inpt = user_input.clone();

                        let dispatch = current_env.dispatch.read().await;
                        let mut handle = dispatch.get_agent_handle(&agent_id).await.unwrap();
                        drop(dispatch);

                        let message = EspxMessage::new_user(&inpt);
                        let ticket = handle.request_stream_completion(message).await.unwrap();

                        let noti = current_env
                            .notifications
                            .wait_for_notification(&ticket)
                            .await
                            .unwrap();

                        let stream_handler: &ThreadSafeStreamCompletionHandler =
                            noti.extract_body().try_into().unwrap();
                        let mut handler = stream_handler.lock().await;

                        let mut whole_message = String::new();
                        while let Some(status) = handler
                            .receive(&handle.id, current_env.clone_sender())
                            .await
                        {
                            match status {
                                CompletionStreamStatus::Working(token) => {
                                    whole_message.push_str(&token);

                                    let tmplt =
                                        models::AssistantMessage::from(whole_message.as_str());
                                    tracing::info!(
                                        "Sending assistant message back to client: {}",
                                        tmplt.render().unwrap()
                                    );

                                    let _ = tx.send(Html(tmplt.render().unwrap()));
                                }
                                CompletionStreamStatus::Finished => {
                                    tracing::info!("Finished completion stream")
                                }
                            }
                        }
                        let _ = current_env.finalize_dispatch().await.unwrap();
                    }
                }
                WsRequest::Empty => {}
            }
            tracing::info!("Message should have sent");
        }
    });

    tracing::info!("Both receive and send tasks spawned");

    // If any one of the tasks run to completion, we abort the other.
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    // Send "user left" message (similar to "joined" above).

    // Remove username from map so new clients can take it again.
    // state.user_set.lock().unwrap().remove(&username);
}
