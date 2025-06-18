use serde::Deserialize;
use std::fs;

#[derive(Clone, Debug, Deserialize)]
pub struct FirebaseConfig;

impl FirebaseConfig {
    pub const API_KEY: &'static str = "AIzaSyCtnUhxetVtQ7XdejiJOobzxsRrI8VRkHY";
    pub const AUTH_DOMAIN: &'static str = "uringredients.firebaseapp.com";
    pub const DATABASE_URL: &'static str =
        "https://uringredients-default-rtdb.asia-southeast1.firebasedatabase.app";
    pub const DATABASE: &'static str = "(default)";
    pub const PROJECT_ID: &'static str = "uringredients";
    pub const STORAGE_BUCKET: &'static str = "uringredients.firebasestorage.app";
    pub const MESSAGING_SENDER_ID: &'static str = "1016308517022";
    pub const APP_ID: &'static str = "1:1016308517022:web:920041db2a1c175d69138f";
    pub const MEASUREMENT_ID: &'static str = "G-S8KDBCVNRD";
}

pub fn firestore_url() -> String {
    format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents",
        FirebaseConfig::PROJECT_ID,
        FirebaseConfig::DATABASE
    )
}
