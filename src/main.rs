use axum::{
    http::{HeaderMap, HeaderValue},
    routing::{delete, get, patch, post},
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
        firestore::{firestore::*, modify::*},
        gemini::*,
        timecheck,
    },
    tools::{imagebb, keycrypt},
};

use basic::{handle_get, handle_post};
use firebase::FirebaseConfig;

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
        .route("/", patch(change_get))
        .route("/schedules/{uuid}", patch(firestore_change_schedule));
    // .route("/recipe/{uuid}", patch(firestore_change_recipe));
    let delete_datas = Router::new()
        .route("/", delete(delete_get))
        .route(
            "/schedules/{uuid}/{schedule_key}",
            delete(firestore_delete_schedule),
        )
        .route(
            "/recipes/{uuid}/{recipe_key}",
            delete(firestore_delete_recipe),
        );

    let add_datas = Router::new()
        .route("/", patch(add_get))
        .route(
            "/schedules/{uuid}",
            patch(firestore_add_schedule),
        )
        .route(
            "/recipes/{uuid}",
            patch(firestore_add_recipe),
        );

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
        .nest("/delete", delete_datas)
        .nest("/add", add_datas)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("localhost:4444")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
