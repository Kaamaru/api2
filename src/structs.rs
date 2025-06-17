use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TheRequest {
    pub text: String,
}
#[derive(Serialize)]

pub struct TheResponse {
    pub text: String,
}
