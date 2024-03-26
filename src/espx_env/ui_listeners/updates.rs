use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use espionox::{
    agents::memory::MessageStack,
    environment::dispatch::{EnvListener, EnvMessage, EnvNotification},
};

pub struct CacheChanged {
    pub(super) agent_id: String,
    pub(super) cache: MessageStack,
}
#[derive(Debug)]
pub struct CacheChangeListener {
    shared_cache_states: Arc<RwLock<HashMap<String, MessageStack>>>,
}

impl CacheChangeListener {
    pub fn new(shared_cache_states: Arc<RwLock<HashMap<String, MessageStack>>>) -> Self {
        Self {
            shared_cache_states,
        }
    }
}

impl EnvListener for CacheChangeListener {
    fn trigger<'l>(
        &self,
        env_message: &'l espionox::environment::dispatch::EnvMessage,
    ) -> Option<&'l espionox::environment::dispatch::EnvMessage> {
        if let EnvMessage::Response(EnvNotification::AgentStateUpdate { .. }) = env_message {
            return Some(env_message);
        }
        None
    }
    #[tracing::instrument(name = "CacheChange Listener Method")]
    fn method<'l>(
        &'l mut self,
        trigger_message: espionox::environment::dispatch::EnvMessage,
        _dispatch: &'l mut espionox::environment::dispatch::Dispatch,
    ) -> espionox::environment::dispatch::listeners::ListenerMethodReturn {
        Box::pin(async move {
            if let EnvMessage::Response(EnvNotification::AgentStateUpdate {
                ref cache,
                ref agent_id,
                ..
            }) = trigger_message
            {
                tracing::info!("Agent: {}", agent_id);
                let mut caches = self.shared_cache_states.write().unwrap();
                caches.insert(agent_id.to_owned(), cache.clone());
                tracing::info!("Sent update");
                return Ok(trigger_message);
            }

            Err(espionox::environment::ListenerError::IncorrectTrigger)
        })
    }
}
