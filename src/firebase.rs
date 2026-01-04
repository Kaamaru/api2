use serde::Deserialize;
use std::env;

// NOTE
// Use your credentials and put this file in /src
// (same level as main.rs)


#[derive(Clone, Debug, Deserialize)]
pub struct GeminiConfig {
    pub api_key: String,
    pub base_url: String,
}

impl GeminiConfig {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        Self {
            api_key: env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY must be set"),
            base_url: env::var("GEMINI_URL").expect("GEMINI_URL must be set"),
        }
    }

   
}





// AppState
#[derive(Clone, Debug)]
pub struct FirebaseConfig {
    pub api_key: String,
    pub auth_domain: String,
    pub database_url: String,
    pub database_name: String,
    pub project_id: String,
    pub storage_bucket: String,
    pub messaging_sender_id: String,
    pub app_id: String,
    pub measurement_id: String,
}

impl FirebaseConfig {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        Self {
            api_key: env::var("FIREBASE_API_KEY").expect("FIREBASE_API_KEY must be set"),
            auth_domain: env::var("FIREBASE_AUTH_DOMAIN")
                .expect("FIREBASE_AUTH_DOMAIN must be set"),
            database_url: env::var("FIREBASE_DATABASE_URL")
                .expect("FIREBASE_DATABASE_URL must be set"),
            database_name: env::var("FIREBASE_DATABASE")
                .unwrap_or_else(|_| "(default)".to_string()),
            project_id: env::var("FIREBASE_PROJECT_ID").expect("FIREBASE_PROJECT_ID must be set"),
            storage_bucket: env::var("FIREBASE_STORAGE_BUCKET")
                .expect("FIREBASE_STORAGE_BUCKET must be set"),
            messaging_sender_id: env::var("FIREBASE_MESSAGING_SENDER_ID")
                .expect("FIREBASE_MESSAGING_SENDER_ID must be set"),
            app_id: env::var("FIREBASE_APP_ID").expect("FIREBASE_APP_ID must be set"),
            measurement_id: env::var("FIREBASE_MEASUREMENT_ID").unwrap_or_else(|_| "".to_string()),

        }
    }

    pub fn firestore_url(&self) -> String {
        format!(
            "https://firestore.googleapis.com/v1/projects/{}/databases/{}/documents",
            self.project_id, self.database_name
        )
    }
}
use axum::extract::FromRef;

impl FromRef<FirebaseConfig> for () {
    fn from_ref(_: &FirebaseConfig) -> Self {
        ()
    }
}
