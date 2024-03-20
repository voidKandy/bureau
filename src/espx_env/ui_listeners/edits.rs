use espionox::{
    agents::memory::{Message, MessageRole, MessageStack},
    environment::dispatch::{EnvListener, EnvMessage, EnvRequest},
};
use tokio::sync::mpsc::Receiver;

pub enum StackEdit {
    EditMessageInCache {
        idx: usize,
        // old_text: &'u str,
        new_text: String,
    },
    RemoveMessageInCache {
        idx: usize,
        // text: String,
    },
    PushMessageToCache {
        message: Message,
    },
}

impl StackEdit {
    fn make_edit(self, cache: &mut MessageStack) {
        match self {
            Self::PushMessageToCache { message } => {
                cache.push(message);
            }
            Self::EditMessageInCache {
                idx,
                // old_text,
                new_text,
            } => {
                if let Some(m) = cache.as_mut().iter_mut().nth(idx) {
                    // if m.content == old_text {
                    m.content = new_text.to_string();
                    // }
                }
            }
            Self::RemoveMessageInCache {
                idx,
                // text
            } => {
                cache.as_mut().remove(idx);
            }
        }
    }
}

pub struct CacheEdit {
    pub agent_id: String,
    pub edit: StackEdit,
}

#[derive(Debug)]
pub struct EditCacheListener {
    receiver: Receiver<CacheEdit>,
}

impl EditCacheListener {
    pub fn new(receiver: Receiver<CacheEdit>) -> Self {
        Self { receiver }
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
            while let Some(change) = self.receiver.recv().await {
                if let Some(agent) = dispatch.get_agent_mut(&change.agent_id).ok() {
                    change.edit.make_edit(&mut agent.cache);
                }
            }
            Ok(trigger_message)
        })
    }
}
