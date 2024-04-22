pub mod models;
use crate::{AppState, SharedState};
use anyhow::anyhow;
use askama::Template;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
};
use espionox::{
    environment::dispatch::ThreadSafeStreamCompletionHandler,
    language_models::openai::completions::streaming::CompletionStreamStatus,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::{broadcast::Sender, RwLockWriteGuard};
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq)]
enum WsRequest {
    PromptAgent { user_input: String },
    // NewChat { chat_name: String, agent: Agent },
    Empty,
}

#[derive(Debug)]
struct WsHxTrigger {
    agent_id: String,
    // env_id: String,
}

struct WsRequestHandler {
    req: WsRequest,
    trigger: WsHxTrigger,
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

impl WsHxTrigger {
    fn try_from_trigger_and_name(hx_trigger: String, hx_trigger_name: String) -> Option<Self> {
        match hx_trigger.as_str() {
            "user-input-form" => {
                if let Some(agent_id) = hx_trigger_name.strip_suffix("-agent-form") {
                    // if let Some(, agent_id)) = env_and_agent_id
                    //     .split_once('-')
                    //     .and_then(|(e, a)| Some((e.to_owned(), a.to_owned())))
                    // {
                    //     return Some(Self { agent_id, env_id });
                    // }

                    return Some(Self {
                        agent_id: agent_id.to_owned(),
                    });
                }
                None
            }
            _ => None,
        }
    }
}

impl TryFrom<Message> for WsRequestHandler {
    type Error = anyhow::Error;
    fn try_from(message: Message) -> Result<Self, Self::Error> {
        if let Message::Text(text) = message {
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

            let trigger =
                WsHxTrigger::try_from_trigger_and_name(hx_trigger, hx_trigger_name).unwrap();

            if let Some(obj) = json_rq.as_object_mut() {
                obj.remove("HEADERS");
            }
            let req = WsRequest::try_from(json_rq).expect("Failed to get message from json");

            tracing::info!(
                "Request deserialized: {:?}\n With trigger: {:?}",
                req,
                trigger
            );

            return Ok(Self { req, trigger });
        }
        Err(anyhow!("Incorrect message type"))
    }
}

impl WsRequestHandler {
    async fn handle(self, mut state: RwLockWriteGuard<'_, AppState>, tx: Sender<Html<String>>) {
        match self.req {
            WsRequest::PromptAgent { user_input } => {
                if !state.env_state.has_handle() {
                    state.env_state.spawn().unwrap();
                }

                let ticket = {
                    let agent_handle = state
                        .env_state
                        .get_agent_handle(&self.trigger.agent_id)
                        .expect("Couldn't get agent handle");

                    agent_handle
                        .request_stream_completion(espionox::agents::memory::Message::new_user(
                            &user_input,
                        ))
                        .await
                        .expect("Why did I fail to request a stream handle?")
                };

                let env_handle = state
                    .env_state
                    .env_handle()
                    .expect("Why can't I get env handle?");

                let noti = env_handle
                    .wait_for_notification(&ticket)
                    .await
                    .expect("Why did I not get Noti?");

                let stream: &ThreadSafeStreamCompletionHandler =
                    noti.extract_body().try_into().unwrap();
                let mut stream = stream.lock().await;

                let mut whole_message = String::new();
                while let Some(status) = stream
                    .receive(&self.trigger.agent_id, env_handle.new_sender())
                    .await
                {
                    match status {
                        CompletionStreamStatus::Working(token) => {
                            whole_message.push_str(&token);

                            let tmplt = models::AssistantMessage::from(whole_message.as_str());
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
            }
            WsRequest::Empty => {}
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
        while let Some(Ok(message)) = receiver.next().await {
            debug!("MESSAGE RECIEVED BY WS: {:?}", message);
            let state_write = state.write().await;
            let tx = state_write.tx.clone();

            let ws_handler = WsRequestHandler::try_from(message).unwrap();

            // let user_message_option: Option<UserMessage> = (&ws_message).try_into().ok();
            // if let Some(msg) = user_message_option {
            //     tracing::info!(
            //         "Sending user message back to client: {}",
            //         msg.render().unwrap()
            //     );
            //     let _ = tx.send(Html(msg.render().unwrap()));
            // }
            ws_handler.handle(state_write, tx).await;
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
