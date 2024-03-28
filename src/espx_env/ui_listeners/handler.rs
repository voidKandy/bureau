use super::listener::{CacheEdit, UiUpdatesListener};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
};

use anyhow::anyhow;
use espionox::{agents::memory::MessageStack, environment::Environment};

#[derive(Debug)]
pub struct UiListenerHandler {
    cache_states: Arc<RwLock<HashMap<String, MessageStack>>>,
    cache_changes: Arc<RwLock<VecDeque<CacheEdit>>>,
    listener: Option<UiUpdatesListener>,
}

impl UiListenerHandler {
    pub async fn new(agent_tup_vec: Vec<(&str, MessageStack)>) -> Self {
        let mut states = HashMap::new();
        for (id, cache) in agent_tup_vec {
            states.insert(id.to_owned(), cache);
        }
        let cache_states = Arc::new(RwLock::new(states));
        let cache_changes = Arc::new(RwLock::new(VecDeque::new()));
        let listener = Some(UiUpdatesListener::new(
            Arc::clone(&cache_changes),
            Arc::clone(&cache_states),
        ));

        Self {
            cache_states,
            cache_changes,
            listener,
        }
    }

    pub fn get_state_of_agent(&self, id: &str) -> Option<MessageStack> {
        let states = self.cache_states.read().unwrap();
        states.get(id).cloned()
    }

    /// Pushes to changes and pre-emptively updates cache state
    #[tracing::instrument(name = "Push change and update cache state", skip(self))]
    pub fn push_to_changes(&mut self, edit: CacheEdit) -> Result<(), anyhow::Error> {
        tracing::info!("getting states write lock");
        let mut states = match self.cache_states.write() {
            Ok(s) => Some(s),
            Err(err) => {
                tracing::error!("ERROR GETTING WRITE LOCK: {:?}", err);
                None
            }
        }
        .unwrap();

        let mut agent_mem = states
            .remove(&edit.agent_id)
            .ok_or(anyhow!("No agent by edit's given id"))?;
        tracing::warn!("Removed agent memory from states");

        let edit_clone = edit.clone();

        edit_clone.edit.make_edit(&mut agent_mem);

        tracing::info!("Edit to agent memory has been made, re-inserting");
        states.insert(edit_clone.agent_id, agent_mem.clone());

        self.cache_changes.write().unwrap().push_back(edit);
        Ok(())
    }

    pub async fn insert_my_listener(&mut self, env: &mut Environment) -> Result<(), anyhow::Error> {
        env.insert_listener(self.listener.take().ok_or(anyhow!("No edit listener!!"))?)
            .await?;
        Ok(())
    }
}
