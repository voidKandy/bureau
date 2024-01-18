use askama::Template;

#[derive(Template)]
#[template(path = "websocket/user_message.html")]
pub struct UserMessage<'a> {
    pub content: &'a str,
}

#[derive(Template)]
#[template(path = "websocket/assistant_message.html")]
pub struct AssistantMessage<'a> {
    pub content: &'a str,
}
