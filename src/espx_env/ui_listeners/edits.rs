use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use espionox::{
    agents::memory::{Message, MessageStack},
    environment::dispatch::{EnvListener, EnvMessage, EnvRequest},
};

#[derive(Debug)]
pub enum StackEdit {
    EditMessageInCache { idx: usize, new_text: String },
    RemoveMessageInCache { idx: usize },
    PushMessageToCache { message: Message },
}

impl StackEdit {
    fn make_edit(self, cache: &mut MessageStack) {
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

#[derive(Debug)]
pub struct CacheEdit {
    pub agent_id: String,
    pub edit: StackEdit,
}

#[derive(Debug)]
pub struct EditCacheListener {
    shared_cache_changes: Arc<RwLock<VecDeque<CacheEdit>>>,
}

impl EditCacheListener {
    pub fn new(shared_cache_changes: Arc<RwLock<VecDeque<CacheEdit>>>) -> Self {
        Self {
            shared_cache_changes,
        }
    }
}

impl EnvListener for EditCacheListener {
    fn trigger<'l>(
        &self,
        env_message: &'l espionox::environment::dispatch::EnvMessage,
    ) -> Option<&'l espionox::environment::dispatch::EnvMessage> {
        if let EnvMessage::Request(r) = env_message {
            match r {
                EnvRequest::PushToCache { .. }
                | EnvRequest::GetCompletion { .. }
                | EnvRequest::GetCompletionStreamHandle { .. } => {
                    return Some(env_message);
                }
                _ => {}
            }
        }
        None
    }
    fn method<'l>(
        &'l mut self,
        trigger_message: espionox::environment::dispatch::EnvMessage,
        dispatch: &'l mut espionox::environment::dispatch::Dispatch,
    ) -> espionox::environment::dispatch::listeners::ListenerMethodReturn {
        Box::pin(async move {
            let mut cache = self.shared_cache_changes.write().unwrap();

            while let Some(change) = cache.pop_front() {
                if let Some(agent) = dispatch.get_agent_mut(&change.agent_id).ok() {
                    change.edit.make_edit(&mut agent.cache);
                }
            }
            Ok(trigger_message)
        })
    }
}
