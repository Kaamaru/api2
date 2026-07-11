#![allow(warnings)]
use axum::{
    http::{HeaderMap, HeaderValue},
    routing::{delete, get, patch, post},
    Router,
};
use reqwest::Method;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task;
use tokio::sync::RwLock;
use dashmap::DashSet;
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
    pub mod batch_detect;
    pub mod classify_mock;
    pub mod firestore;
    pub mod gemini;
    pub mod getimage;
    pub mod hardware;
    pub mod line;
    pub mod sendmail;
    pub mod timecheck;
    pub mod recipe_search;
}

mod tools {
    pub mod imagebb;
    pub mod keycrypt;
}

mod tests {
    pub mod dev;
    pub mod api_contract;
}

use crate::{
    tasks::{
        basic,
        batch_detect::batch_detect_handler,
        firestore::{firestore::*, household::*, household_extended::*, modify::*, notify::*, grocery::*, people::*, recipe::*},
        gemini::*,
        getimage::*,
        hardware::{device::*, tractor::tract_test,requests::*},
        line::{generate_link_code_handler, line_status_handler, line_webhook_handler},
        timecheck,
        recipe_search::{
            AppState, add_recipe_info_to_list_handler, add_recipe_to_list_handler, generate_recipe_handler,
            get_recipe_by_id_handler, search_recipe_handler,
        },
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
    
    // Initialize AppState with cache from Firestore
    let cache_data = match load_all_dishnames(&firebase_config).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("⚠️ Failed to load cache from Firestore: {}", e);
            Vec::new()
        }
    };
    
    let app_state = AppState {
        cache: Arc::new(RwLock::new(cache_data)),
        generating: Arc::new(DashSet::new()),
        firebase_config: Arc::new(firebase_config.clone()),
    };
    
    task::spawn(async {
        timeloop().await;
    });
// tract_test();
    keycrypt();
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
        ).route(
            "/devices/{userid}",
            post(add_device_record).delete(delete_device_records),
        ).route(
            "/requests/{userid}",
            post(add_request_record).delete(delete_request_records).patch(accept_request_record),
        ).route(
            "/{userid}",
            get(get_hardware_records),
        );


    let notify = Router::new()
        .route("/on/{uuid}/{schedule_id}", patch(firestore_set_notify_on))
        .route("/off/{uuid}/{schedule_id}", patch(firestore_set_notify_off))
        .route("/get_notify/{uuid}", get(firestore_get_notify_schedule_all))
        .route("/test/{uid}", post(firestore_test_notification))
        // .route("/get_user_notify", get(firestore_get_notified_users_schedules));
        ;

    let grocery = Router::new()
        .route("/{userid}", get(get_grocery_lists).post(create_grocery_list))
        .route("/{userid}/{list_id}", get(get_grocery_list).delete(delete_grocery_list))
        .route("/{userid}/{list_id}/items", patch(add_grocery_item))
        .route("/{userid}/{list_id}/items/{item_id}", patch(update_grocery_item).delete(delete_grocery_item))
        .route("/{userid}/{list_id}/items/{item_id}/to-schedule", post(convert_grocery_to_schedule));

    let people = Router::new()
        .route("/{userid}", get(get_people_profiles).post(create_person_profile))
        .route("/{userid}/{person_id}", patch(update_person_profile).delete(delete_person_profile));

    let households = Router::new()
        .route("/", post(create_household))
        .route("/my", get(get_my_household))
        .route("/{household_id}/generate-invite", post(generate_household_invite))
        .route("/redeem-invite", post(redeem_household_invite))
        .route("/{household_id}/members/{member_uid}", delete(remove_member))
        .route("/{household_id}/leave", post(leave_household))
        .route("/{household_id}/schedules", get(list_household_schedules).post(add_household_schedule))
        .route("/{household_id}/schedules/{schedule_id}", patch(update_household_schedule).delete(delete_household_schedule))
        .route("/{household_id}/grocery", get(list_household_grocery_lists).post(create_household_grocery_list))
        .route("/{household_id}/grocery/{list_id}", delete(delete_household_grocery_list))
        .route("/{household_id}/grocery/{list_id}/items", post(add_household_grocery_item))
        .route("/{household_id}/grocery/{list_id}/items/{item_id}", patch(update_household_grocery_item).delete(delete_household_grocery_item));

    let line = Router::new()
        .route("/link-code", post(generate_link_code_handler))
        .route("/status/{userid}", get(line_status_handler))
        .route("/webhook", post(line_webhook_handler));

    let search = Router::new()
        .route("/recipes", get(search_recipe_handler))
        .route("/recipes/{id}", get(get_recipe_by_id_handler))
        .route("/recipes/{id}/add-to-list", post(add_recipe_to_list_handler))
        .route("/recipes/create-list/from-info", post(add_recipe_info_to_list_handler))
        .route("/recipes/generate", post(generate_recipe_handler))
        .with_state(app_state.clone());

    let app: Router = Router::new()
        .route("/", get(handle_get).post(handle_post))
        .route("/chat", post(chat_handler_with_retry))
        .route("/classify", post(classify_image_handler))
        .route("/batch-detect", post(batch_detect_handler))
        .route("/user/{uid}", get(handle_get_user_by_uid))
        .route("/data", get(handle_get_data))
    .route("/echo", get(async move ||{ println!("echo!");"Echo!"}))
        .route("/user-summary", get(handle_get_user_summary))
        .route("/checktime", get(firestore_get_users_and_schedules))
        .nest("/feedbacks", feedbacks_routes)
        .nest("/change", change_datas)
        .nest("/delete", delete_datas)
        .nest("/add", add_datas)
        .nest("/notify", notify)
        .nest("/grocery", grocery)
        .nest("/people", people)
        .nest("/households", households)
        .nest("/hardware", hardware)
        .nest("/line", line)
        .nest("/search", search)
        .layer(cors)
        .with_state(firebase_config);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "4444".to_string())
        .parse()
        .unwrap_or(4444);

    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();

    println!("Listening on {}", address);
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
