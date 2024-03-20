pub mod edits;
pub mod updates;

pub use edits::*;
pub use updates::*;

use std::collections::HashMap;

use anyhow::anyhow;
use espionox::{agents::memory::MessageStack, environment::Environment};
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct UiListenerChannel {
    cache_states: HashMap<String, MessageStack>,
    update_receiver: Receiver<CacheChanged>,
    edit_senders: HashMap<String, Sender<CacheEdit>>,

    edit_listener: Option<EditCacheListener>,
    update_listener: Option<CacheChangeListener>,
}

impl UiListenerChannel {
    pub async fn new(agent_tup_vec: Vec<(&str, MessageStack)>) -> Self {
        let edit_channel: (Sender<CacheEdit>, Receiver<CacheEdit>) = tokio::sync::mpsc::channel(20);

        let update_channel: (Sender<CacheChanged>, Receiver<CacheChanged>) =
            tokio::sync::mpsc::channel(20);

        let mut edit_senders = HashMap::new();
        let mut cache_states = HashMap::new();
        for (id, cache) in agent_tup_vec.into_iter() {
            edit_senders.insert(id.to_string(), edit_channel.0.clone());
            cache_states.insert(id.to_string(), cache);
        }
        let edit_listener = Some(EditCacheListener::new(edit_channel.1));
        let update_listener = Some(CacheChangeListener::new(update_channel.0));

        Self {
            cache_states,
            edit_senders,
            update_receiver: update_channel.1,
            edit_listener,
            update_listener,
        }
    }

    pub async fn update_cache_states(&mut self) {
        while let Some(update) = self.update_receiver.recv().await {
            self.cache_states.insert(update.agent_id, update.cache);
        }
    }

    pub fn get_agent_cache(&self, id: &str) -> Option<&MessageStack> {
        self.cache_states.get(id)
    }

    pub async fn insert_my_listeners(
        &mut self,
        env: &mut Environment,
    ) -> Result<(), anyhow::Error> {
        env.insert_listener(
            self.edit_listener
                .take()
                .ok_or(anyhow!("No edit listener!!"))?,
        )
        .await?;

        env.insert_listener(
            self.update_listener
                .take()
                .ok_or(anyhow!("No update listener!!"))?,
        )
        .await?;
        Ok(())
    }

    pub fn get_agent_edit_sender(&mut self, id: &str) -> Option<&mut Sender<CacheEdit>> {
        self.edit_senders.get_mut(id)
    }
}
