use std::collections::HashMap;
pub mod ui_listeners;

use anyhow::anyhow;
use dotenv::dotenv;
use espionox::{
    agents::Agent,
    environment::{agent_handle::AgentHandle, env_handle::EnvHandle, EnvError, Environment},
    language_models::{ModelProvider, LLM},
};

use self::ui_listeners::UiListenerChannel;

pub type EnvStatesMap = HashMap<String, EnvironmentState>;

#[derive(Debug)]
pub struct EnvironmentState {
    pub env: Environment,
    pub ui_channel: UiListenerChannel,
    agent_handles: HashMap<String, AgentHandle>,
    handle: Option<EnvHandle>,
}

pub fn default_env() -> Environment {
    dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY").unwrap();
    let mut map = HashMap::new();
    tracing::info!("Got openai api key: {}", api_key);
    map.insert(ModelProvider::OpenAi, api_key);
    Environment::new(Some("default"), map)
}

pub fn default_agents() -> Vec<(&'static str, Agent)> {
    let default_agent = Agent::new(None, LLM::default_openai());
    let non_default_agent =
        Agent::new(Some("You are the non default agent"), LLM::default_openai());

    vec![
        ("default", default_agent),
        ("non-default", non_default_agent),
    ]
}

impl EnvironmentState {
    pub async fn init() -> Result<Self, anyhow::Error> {
        let agents = default_agents();
        let mut tup_vec = vec![];

        for (id, a) in agents.iter() {
            tup_vec.push((id.to_owned(), a.cache.clone()));
        }

        let mut ui_channel = UiListenerChannel::new(tup_vec).await;
        let mut agent_handles = HashMap::new();

        let mut env = default_env();
        for (id, a) in agents.into_iter() {
            let h = env.insert_agent(Some(id), a).await?;
            agent_handles.insert(id.to_string(), h);
        }
        ui_channel
            .insert_my_listeners(&mut env)
            .await
            .expect("Couldn't insert UI listeners");

        Ok(Self {
            env,
            ui_channel,
            handle: None,
            agent_handles,
        })
    }

    pub fn agent_names(&self) -> Vec<String> {
        self.agent_handles.keys().map(|k| k.to_string()).collect()
    }

    pub fn env_handle(&mut self) -> Result<&mut EnvHandle, EnvError> {
        self.handle.as_mut().ok_or(anyhow!("No handle").into())
    }

    pub fn spawn(&mut self) -> Result<(), EnvError> {
        if self.has_handle() {
            return Err(EnvError::MissingHandleData);
        }
        let handle = self
            .env
            .spawn_handle()
            .expect("Why couldn't I spawn handle?");
        self.handle = Some(handle);
        Ok(())
    }

    pub fn has_handle(&self) -> bool {
        self.handle.is_some()
    }

    pub fn get_agent_handle(&self, id: &str) -> Option<&AgentHandle> {
        self.agent_handles.get(id)
    }
}
