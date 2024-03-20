use anyhow::anyhow;
use askama::Template;
use markdown::to_html;

use super::WsRequest;

#[derive(Template, Debug)]
#[template(path = "websocket/user_message.html")]
pub struct UserMessage {
    pub content: String,
}

#[derive(Template)]
#[template(path = "websocket/assistant_message.html")]
pub struct AssistantMessage {
    pub content: String,
}

impl TryFrom<&WsRequest> for UserMessage {
    type Error = anyhow::Error;
    fn try_from(req: &WsRequest) -> Result<Self, Self::Error> {
        match req {
            WsRequest::PromptAgent { user_input } => {
                let template = UserMessage {
                    // For some reason to_html appends a newline, so we remove it
                    content: to_html(&user_input).trim_matches('\n').to_string(),
                };
                Ok(template)
            }
            WsRequest::Empty => Err(anyhow!("Wrong request type")),
        }
    }
}

impl From<&str> for AssistantMessage {
    fn from(str: &str) -> Self {
        AssistantMessage {
            content: to_html(str),
        }
    }
}
