use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TheRequest {
    pub text: String,
}
#[derive(Serialize)]

pub struct TheResponse {
    pub text: String,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreStringField {
    pub stringValue: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FeedbackFields {
    pub name: FirestoreStringField,
    pub status: FirestoreStringField,
    pub message: FirestoreStringField,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct FeedbackDocument {
    pub fields: FeedbackFields,
}
