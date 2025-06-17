use axum::{
    http::HeaderMap,
    routing::{get, post},
    Router,
};

mod tasks {
    pub mod basic;
    pub mod keycrypt;
}

pub mod structs;

use crate::tasks::*;
use basic::{handle_get, handle_post};
use keycrypt::keycrypt;



#[tokio::main]
async fn main() {
    keycrypt();
    let app = Router::new()
        .route("/", get(handle_get))
        .route("/", post(handle_post));

    let listener = tokio::net::TcpListener::bind("localhost:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
