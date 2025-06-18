use std::fmt::format;

use axum::{
    extract::Json,
    http::{response, HeaderMap, HeaderName, HeaderValue, StatusCode},
    routing::get,
    Json as AxumJson, Router,
};
use crate::firebase::{self, FirebaseConfig};
use serde::{Deserialize, Serialize};
// use reqwest::{Client};
// const BASE_URL: &str = format!("https://firestore.googleapis.com/v1/projects/{}/databases/(default)/documents",FirebaseConfig::PROJECT_ID).as_str();

pub async fn handle_get_data()  {
    let url = format!("{}/users", firebase::firestore_url());
    println!("{}",url);  

    let response = reqwest::get(&url)
        .await
        .expect("Failed to send request");  
    println!("{:?}", response.text().await.expect("Failed to read response text"));

}   