use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
};

use espionox::{
    agents::memory::{Message, MessageStack},
    environment::dispatch::{EnvListener, EnvMessage, EnvNotification, EnvRequest},
};

#[derive(Debug, Clone)]
pub enum StackEdit {
    EditMessageInCache { idx: usize, new_text: String },
    RemoveMessageInCache { idx: usize },
    PushMessageToCache { message: Message },
}

impl StackEdit {
    #[tracing::instrument(name = "Make edit to messagestack")]
    pub(super) fn make_edit(self, cache: &mut MessageStack) {
        match self {
            Self::PushMessageToCache { message } => {
                cache.push(message);
            }
            Self::EditMessageInCache { idx, new_text } => {
                if let Some(m) = cache.as_mut().iter_mut().nth(idx) {
                    m.content = new_text.to_string();
                }
            }
            Self::RemoveMessageInCache { idx } => {
                cache.as_mut().remove(idx);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheEdit {
    pub agent_id: String,
    pub edit: StackEdit,
}

#[derive(Debug)]
pub struct UiUpdatesListener {
    shared_cache_changes: Arc<RwLock<VecDeque<CacheEdit>>>,
    shared_cache_states: Arc<RwLock<HashMap<String, MessageStack>>>,
}

impl UiUpdatesListener {
    pub fn new(
        shared_cache_changes: Arc<RwLock<VecDeque<CacheEdit>>>,
        shared_cache_states: Arc<RwLock<HashMap<String, MessageStack>>>,
    ) -> Self {
        Self {
            shared_cache_changes,
            shared_cache_states,
        }
    }
}

impl EnvListener for UiUpdatesListener {
    fn trigger<'l>(
        &self,
        env_message: &'l espionox::environment::dispatch::EnvMessage,
    ) -> Option<&'l espionox::environment::dispatch::EnvMessage> {
        match env_message {
            EnvMessage::Response(res) => {
                if let EnvNotification::AgentStateUpdate { .. } = res {
                    return Some(env_message);
                }
            }
            EnvMessage::Request(r) => match r {
                EnvRequest::PushToCache { .. }
                | EnvRequest::GetCompletion { .. }
                | EnvRequest::GetAgentState { .. }
                | EnvRequest::GetCompletionStreamHandle { .. } => {
                    return Some(env_message);
                }
                _ => {}
            },
            _ => {}
        }
        None
    }
    fn method<'l>(
        &'l mut self,
        trigger_message: espionox::environment::dispatch::EnvMessage,
        dispatch: &'l mut espionox::environment::dispatch::Dispatch,
    ) -> espionox::environment::dispatch::listeners::ListenerMethodReturn {
        Box::pin(async move {
            match trigger_message {
                EnvMessage::Response(ref res) => {
                    if let EnvNotification::AgentStateUpdate {
                        ref cache,
                        ref agent_id,
                        ..
                    } = res
                    {
                        tracing::info!("Agent: {}", agent_id);
                        let mut caches = self.shared_cache_states.write().unwrap();
                        caches.insert(agent_id.to_owned(), cache.clone());
                        tracing::info!("Sent update");
                        return Ok(trigger_message);
                    }
                }
                EnvMessage::Request(ref r) => match r {
                    EnvRequest::PushToCache { .. }
                    | EnvRequest::GetCompletion { .. }
                    | EnvRequest::GetAgentState { .. }
                    | EnvRequest::GetCompletionStreamHandle { .. } => {
                        let mut cache_changes = self.shared_cache_changes.write().unwrap();

                        while let Some(change) = cache_changes.pop_front() {
                            if let Some(agent) = dispatch.get_agent_mut(&change.agent_id).ok() {
                                change.edit.make_edit(&mut agent.cache);
                            }
                        }
                        return Ok(trigger_message);
                    }
                    _ => {}
                },
                _ => {}
            }

            Err(espionox::environment::ListenerError::IncorrectTrigger)
        })
    }
}
