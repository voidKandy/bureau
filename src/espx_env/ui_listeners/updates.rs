use espionox::{
    agents::memory::MessageStack,
    environment::dispatch::{EnvListener, EnvMessage, EnvNotification},
};
use tokio::sync::mpsc::Sender;

pub struct CacheChanged {
    pub(super) agent_id: String,
    pub(super) cache: MessageStack,
}
#[derive(Debug)]
pub struct CacheChangeListener {
    sender: Sender<CacheChanged>,
}

impl CacheChangeListener {
    pub fn new(sender: Sender<CacheChanged>) -> Self {
        Self { sender }
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
                self.sender
                    .send(CacheChanged {
                        agent_id: agent_id.to_owned(),
                        cache: cache.to_owned(),
                    })
                    .await
                    .expect("Failed to send cache change");
                return Ok(trigger_message);
            }

            Err(espionox::environment::ListenerError::IncorrectTrigger)
        })
    }
}
