use axum::{
    http::HeaderMap,
    routing::{get, post},
    Router,
};
use tokio::task;

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
    pub mod sendmail;
    pub mod timecheck;
}

mod tools {
    pub mod imagebb;
    pub mod keycrypt;
}

use crate::{
    firebase::firestore_url,
    tasks::{
        basic,
        firestore::{self},
        timecheck,
    },
    tools::{imagebb, keycrypt},
};

use basic::{handle_get, handle_post};
use firebase::FirebaseConfig;
use firestore::{
    handle_get_data, handle_get_feedback_devread, firestore_get_users_and_schedules,
    handle_post_feedback,
};
use imagebb::upload_to_imgbb;
use keycrypt::keycrypt;
use timecheck::timeloop;

#[tokio::main]
async fn main() {
    task::spawn(async {
        timeloop().await;
    });

    println!("{}", firestore_url());
    keycrypt();

    let feedbacks_routes = Router::new()
        .route("/", post(handle_post_feedback))
        .route("/devread/{uuid}", get(handle_get_feedback_devread));

    let app = Router::new()
        .route("/", get(handle_get).post(handle_post))
        .route("/data", get(handle_get_data))
        .route("/echo", get(async move || "Echo!"))
        .route("/checktime", get(firestore_get_users_and_schedules))
        .nest("/feedbacks", feedbacks_routes);
    let listener = tokio::net::TcpListener::bind("localhost:4444")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
