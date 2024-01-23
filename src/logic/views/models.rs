use anyhow::anyhow;
use askama::Template;
use espionox::environment::{
    agent::memory::{Message, MessageRole},
    dispatch::{EnvMessage, EnvNotification, EnvRequest},
};
use nom::{
    self,
    bytes::complete::{take, take_till, take_until},
    sequence::delimited,
    IResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate;

#[derive(Template)]
#[template(path = "layout.html")]
pub struct LayoutTemplate<'a> {
    // pub path: &'a str,
    // pub params: &'a str,
    pub environment_names: Option<Vec<String>>,
    pub path_and_params: Option<(&'a str, &'a str)>,
}

#[derive(Template)]
#[template(path = "env_view.html")]
pub struct EnvView<'a> {
    pub id: &'a str,
    pub agent_names: Vec<String>,
}

#[derive(Template)]
#[template(path = "agent_view.html")]
pub struct AgentView<'a> {
    pub id: &'a str,
    pub parent_id: &'a str,
}

#[derive(Template)]
#[template(path = "chat_history.html")]
pub struct ChatHistory {
    pub env_id: String,
    pub agent_id: String,
    pub messages: Vec<MessageRender>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct MessageRender {
    pub class: String,
    pub content: String,
    pub markdown: Option<String>,
}

impl Into<Message> for MessageRender {
    fn into(self) -> Message {
        match self.class.as_str() {
            "user-message" => Message::new_user(&self.content),
            "assistant-message" => Message::new_assistant(&self.content),
            "system-message" => Message::new_system(&self.content),
            _ => unreachable!(),
        }
    }
}

impl MessageRender {
    fn role(&self) -> &str {
        match self.class.as_str() {
            "user-message" => "user",
            "assistant-message" => "assistant",
            "system-message" => "system",
            _ => unreachable!(),
        }
    }
}

impl From<&Message> for MessageRender {
    fn from(m: &Message) -> Self {
        let mut class = String::new();
        class.push_str({
            match m.role {
                MessageRole::User => "user-message",
                MessageRole::Assistant => "assistant-message",
                _ => "system-message",
            }
        });
        let content = &m.content;
        let markdown = match parse_for_markdown(&content) {
            Ok((_, o)) => {
                if !o.is_empty() {
                    Some(o.to_string())
                } else {
                    None
                }
            }
            Err(_) => None,
        };

        Self {
            class,
            content: content.to_string(),
            markdown,
        }
    }
}

pub fn parse_for_markdown(input: &str) -> IResult<&str, &str> {
    let till_md = |c| -> bool {
        static mut ACC: u8 = 0;

        unsafe {
            if ACC == 3 {
                true
            } else if c == '`' {
                ACC += 1;
                false
            } else {
                false
            }
        }
    };
    let (r, o) = delimited(take_till(till_md), take_until("```"), take(3usize))(input)?;
    Ok((r, o))
}
