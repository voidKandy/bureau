use askama::Template;
use espionox::agents::memory::{Message, MessageRole};
use markdown::to_html;
use serde::{Deserialize, Serialize};

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate;

#[derive(Template)]
#[template(path = "layout.html")]
pub struct LayoutTemplate<'a> {
    pub agent_names: Option<Vec<String>>,
    pub path_and_params: Option<(&'a str, &'a str)>,
}

#[derive(Template)]
#[template(path = "agent_view.html")]
pub struct AgentView<'a> {
    pub agent_id: &'a str,
}

#[derive(Template)]
#[template(path = "chat_history.html")]
pub struct ChatHistory {
    pub agent_id: String,
    pub messages: Vec<MessageRender>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct MessageRender {
    pub class: String,
    pub content: String,
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
    #[tracing::instrument(name = "Converting from EspxMessage to MessageRender")]
    fn from(m: &Message) -> Self {
        let mut class = String::new();
        class.push_str({
            match m.role {
                MessageRole::User => "user-message",
                MessageRole::Assistant => "assistant-message",
                _ => "system-message",
            }
        });

        let content = to_html(&m.content);
        let sani = content.replace("\\n", "<br>");

        Self {
            class,
            content: sani,
        }
    }
}
