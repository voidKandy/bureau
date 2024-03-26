use super::websocket as ws;
use crate::{
    patches,
    // data_controllers,
    views::{self, models::LayoutTemplate},
    SharedState,
};

use askama::Template;
use axum::{
    http::Request,
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, patch},
    Router,
};
use axum_htmx::extractors::HxRequest;

use tower_http::services::ServeDir;

pub fn main_router() -> Router<SharedState> {
    let websocket_routes = init_ws_routes();
    let agent_routes = init_agent_routes();
    Router::new()
        .route("/", get(views::templates::index))
        .nest("/:agent_id", agent_routes)
        .layer(middleware::from_fn(non_hx_request_middleware))
        .nest("/ws", websocket_routes)
        .nest_service("/static", ServeDir::new("static"))
}

fn init_agent_routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(views::partials::agent_view))
        .route("/history", get(views::partials::history))
        .route("/message_change/:index", patch(patches::message_change))
        .route("/add_message", patch(patches::add_message))
        .route("/add_message_form", get(patches::add_message_form))
}

fn init_ws_routes() -> Router<SharedState> {
    Router::new().route("/", get(ws::websocket_handler))
}

async fn non_hx_request_middleware<B>(
    HxRequest(hx_req): HxRequest,
    req: Request<B>,
    next: Next<B>,
) -> Response {
    let uri = req.uri();
    // let headers = req.headers();
    // tracing::info!("Hit middleware from: {:?}", uri);
    // tracing::info!("Headers: {:?}", headers);
    let params = uri.query().unwrap_or("");
    let path = uri.path();
    // tracing::info!("Path: {:?}\nParams: {:?}", path, params);

    if !hx_req {
        tracing::info!("HxRequest header not present, middleware returning HTML...");
        let path_and_params = Some((path, params));
        let template = LayoutTemplate {
            agent_names: None,
            path_and_params,
        };
        return Html(template.render().unwrap()).into_response();
    }

    tracing::info!("HxRequest header present, passing through middleware...");
    next.run(req).await.into()
}
