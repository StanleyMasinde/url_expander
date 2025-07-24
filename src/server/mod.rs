use crate::{expander, proxy, request};
use std::{collections::HashMap, env::args};

use axum::{
    Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use reqwest::{Client, StatusCode};

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

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/proxy", get(proxy_url))
        .with_state(state);

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
        let res = expander::expand_url(url.to_string(), client).await;
        (StatusCode::OK, res)
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
        (
            StatusCode::OK,
            proxy::return_preview_html(url.to_string(), client)
                .await
                .unwrap(),
        )
    } else {
        (StatusCode::BAD_REQUEST, "URL parameter missing".to_string())
    }
}
