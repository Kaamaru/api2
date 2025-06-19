use axum::{
    http::HeaderMap,
    routing::{get, post},
    Router,
};

// main.rs or lib.rs

mod firebase;
mod parse;

mod structures {
    pub mod models;
    pub mod structs;
}

mod tasks {
    pub mod basic;
    pub mod firestore;
    pub mod gemini;
}

mod tools {
    pub mod imagebb;
    pub mod keycrypt;
}

use crate::{
    firebase::firestore_url,
    tasks::{basic, firestore},
    tools::{imagebb, keycrypt},
};

use basic::{handle_get, handle_post};
use firebase::FirebaseConfig;
use firestore::{handle_get_data, handle_get_feedback_devread, handle_post_feedback};
use imagebb::upload_to_imgbb;
use keycrypt::keycrypt;

#[tokio::main]
async fn main() {
    println!("{}", firestore_url());
    keycrypt();

    let feedbacks_routes = Router::new()
        .route("/", post(handle_post_feedback))
        .route("/devread/{uuid}", get(handle_get_feedback_devread));

    let app = Router::new()
        .route("/", get(handle_get).post(handle_post))
        .route("/data", get(handle_get_data))
        .route("/echo", get(async move || "Echo!"))
        .nest("/feedbacks", feedbacks_routes);
    let listener = tokio::net::TcpListener::bind("localhost:4444")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
