use serde::Deserialize;

// NOTE
// Use your credentials and put this file in /src
// (same level as main.rs)

#[derive(Clone, Debug, Deserialize)]
pub struct FirebaseConfig;

impl FirebaseConfig {
    pub const API_KEY: &'static str = "APIKEY";
    pub const AUTH_DOMAIN: &'static str = "AUTHDOMAIN";
    pub const DATABASE_URL: &'static str = "DATABASEURL";
    pub const DATABASE: &'static str = "DATABASE";
    pub const PROJECT_ID: &'static str = "PROJECTID";
    pub const STORAGE_BUCKET: &'static str = "STORAGEBUCKET";
    pub const MESSAGING_SENDER_ID: &'static str = "MESSAGINGSENDERID";
    pub const APP_ID: &'static str = "APPID";
    pub const MEASUREMENT_ID: &'static str = "MEASUREMENTID";
}

pub fn firestore_url() -> String {
    format!(
        "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents",
        FirebaseConfig::PROJECT_ID,
        FirebaseConfig::DATABASE
    )
}
