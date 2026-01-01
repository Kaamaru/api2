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
    pub mod getimage;
    pub mod hardware;
    pub mod sendmail;
    pub mod timecheck;
}

mod tools {
    pub mod imagebb;
    pub mod keycrypt;
}

use crate::{
    tasks::{
        basic,
        firestore::{firestore::*, modify::*, notify::*},
        gemini::*,
        getimage::*,
        hardware::device::*,
        timecheck,
    },
    tools::{imagebb, keycrypt},
};

use basic::{handle_get, handle_post};

use imagebb::upload_to_imgbb;
use keycrypt::keycrypt;
use timecheck::timeloop;

pub use firebase::FirebaseConfig;

use crate::tasks::sendmail::do_expiration_mail;

#[tokio::main]
async fn main() {
    let firebase_config = FirebaseConfig::from_env();
    task::spawn(async {
        timeloop().await;
    });

    println!("{}", firebase_config.firestore_url());
    keycrypt();
    println!("\n\n{:?}\n\n", firestore_get_notified_users_schedules().await);
    do_expiration_mail().await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let feedbacks_routes = Router::new()
        .route("/", post(handle_post_feedback))
        .route("/devread/{uuid}", get(handle_get_feedback_devread));
    let change_datas = Router::new()
        .route("/", patch(change_get))
        .route("/schedules/{uuid}", patch(firestore_change_schedule))
        .route(
            "/displayname/{uuid}/{newname}",
            patch(firestore_change_displayname),
        );
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
        .route("/schedules/{uuid}", patch(firestore_add_schedule))
        .route("/recipes/{uuid}", patch(firestore_add_recipe));

    let hardware = Router::new()
        .route("/connect/{hardware_secrets}", post(connect_device_to_user))
        .route(
            "/send_image/{hardware_secrets}",
            post(recieve_image_bytes_esp32),
        );

    let notify = Router::new()
        .route("/on/{uuid}/{schedule_id}", patch(firestore_set_notify_on))
        .route("/off/{uuid}/{schedule_id}", patch(firestore_set_notify_off))
        .route("/get_notify/{uuid}", get(firestore_get_notify_schedule_all))
        // .route("/get_user_notify", get(firestore_get_notified_users_schedules));
        ;

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
        .nest("/notify", notify)
        .nest("/hardware", hardware)
        .layer(cors)
        .with_state(firebase_config);

    let listener = tokio::net::TcpListener::bind("localhost:4444")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

use dotenv::dotenv;
// Access env vars
pub fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
}
pub fn secrets_setup() {
    dotenv().ok();

    // Install ring provider for rustls 0.23
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install default crypto provider");
}
