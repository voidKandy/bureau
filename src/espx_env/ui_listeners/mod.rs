pub mod edits;
pub mod updates;

pub use edits::*;
pub use updates::*;

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
};

use anyhow::anyhow;
use espionox::{agents::memory::MessageStack, environment::Environment};

#[derive(Debug)]
pub struct UiListenerHandler {
    pub cache_states: Arc<RwLock<HashMap<String, MessageStack>>>,
    pub cache_changes: Arc<RwLock<VecDeque<CacheEdit>>>,
    edit_listener: Option<EditCacheListener>,
    update_listener: Option<CacheChangeListener>,
}

impl UiListenerHandler {
    pub async fn new(agent_tup_vec: Vec<(&str, MessageStack)>) -> Self {
        let mut states = HashMap::new();
        for (id, cache) in agent_tup_vec {
            states.insert(id.to_owned(), cache);
        }
        let cache_states = Arc::new(RwLock::new(states));
        let cache_changes = Arc::new(RwLock::new(VecDeque::new()));
        let edit_listener = Some(EditCacheListener::new(Arc::clone(&cache_changes)));
        let update_listener = Some(CacheChangeListener::new(Arc::clone(&cache_states)));

        Self {
            cache_states,
            cache_changes,
            edit_listener,
            update_listener,
        }
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
}
