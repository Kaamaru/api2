use axum::{
    http::HeaderMap,
    routing::{get, post},
    Router,
};

mod tasks {
    pub mod basic;
    pub mod keycrypt;
    pub mod firestore;
}

pub mod structs;
pub mod firebase;

use crate::{firebase::firestore_url, tasks::*};
use basic::{handle_get, handle_post};
use keycrypt::keycrypt;
use firestore::{handle_get_data};
use firebase::FirebaseConfig;

#[tokio::main]
async fn main() {
    println!("{}",firestore_url());  
    keycrypt();
    let app = Router::new()
        .route("/", get(handle_get).post(handle_post))
        .route("/data",get(handle_get_data) );

    let listener = tokio::net::TcpListener::bind("localhost:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
