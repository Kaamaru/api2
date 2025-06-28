use axum::{
    http::{HeaderMap, HeaderValue},
    routing::{get, patch, post},
    Router,
};
use reqwest::Method;
use tokio::task;
use tower_http::cors::{Any, CorsLayer};

mod firebase;
mod parse;

mod structures {
    pub mod models;
    pub mod structs;
    pub mod webstructs;
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
        firestore::{self, handle_get_user_by_uid, handle_get_user_summary},
        gemini::*,
        timecheck,
    },
    tools::{imagebb, keycrypt},
};

use basic::{handle_get, handle_post};
use firebase::FirebaseConfig;
use firestore::{
    firestore_change_recipe, firestore_change_schedule, firestore_get_users_and_schedules,
    handle_get_data, handle_get_feedback_devread, handle_post_feedback,
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
    let cors = CorsLayer::new()
        .allow_origin(Any) // You can also use `allow_origin("https://yourfrontend.com".parse().unwrap())`
        .allow_methods(Any)
        .allow_headers(Any);
    let feedbacks_routes = Router::new()
        .route("/", post(handle_post_feedback))
        .route("/devread/{uuid}", get(handle_get_feedback_devread));
    let change_datas = Router::new()
        .route("/schedules/{uuid}", patch(firestore_change_schedule))
        .route("/recipe/{uuid}", patch(firestore_change_recipe));

    let app = Router::new()
        .route("/", get(handle_get).post(handle_post))
        .route("/chat", post(chat_handler_with_retry))
        .route("/user/{uid}", get(handle_get_user_by_uid))
        .route("/data", get(handle_get_data))
        .route("/echo", get(async move || "Echo!"))
        .route("/user-summary", get(handle_get_user_summary))
        .route("/checktime", get(firestore_get_users_and_schedules))
        .nest("/feedbacks", feedbacks_routes)
        .nest("/change", change_datas)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("localhost:4444")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
