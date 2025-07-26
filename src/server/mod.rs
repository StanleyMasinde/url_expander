use crate::{expander, proxy, request, utils::handle_reqwest_error};
use std::{collections::HashMap, env::args};

use axum::{
    Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use reqwest::{Client, Method, StatusCode};
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Clone)]
struct AppState {
    client: Client,
}

pub async fn run() {
    let default_port = String::from("3000");
    let args: Vec<String> = args().collect();
    let port = args.get(1).unwrap_or(&default_port);
    let address = format!("127.0.0.1:{port}");

    let client = request::create_reqwest();
    let state = AppState { client };

    // Layers
    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(AllowOrigin::mirror_request());

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/proxy", get(proxy_url))
        .with_state(state)
        .layer(cors);

    println!("Server running on http://{}", &address);
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let client = state.client;

    if let Some(url) = params.get("url") {
        match expander::expand_url(url.to_string(), client).await {
            Ok(url) => (StatusCode::OK, url),
            Err(error) => handle_reqwest_error(error),
        }
    } else {
        (StatusCode::BAD_REQUEST, "URL parameter missing".to_string())
    }
}

async fn proxy_url(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let client = state.client;

    if let Some(url) = params.get("url") {
        match proxy::return_preview_html(url.to_string(), client).await {
            Ok(html) => (StatusCode::OK, html.to_string()),
            Err(error) => handle_reqwest_error(error),
        }
    } else {
        (StatusCode::BAD_REQUEST, "URL parameter missing".to_string())
    }
}
