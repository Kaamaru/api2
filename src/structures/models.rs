use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub history: serde_json::Value,
    pub prompt: serde_json::Value,
    #[serde(rename = "promptlanguage")]
    pub prompt_language: String,
}


#[derive(Serialize)]
pub struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<SystemInstruction>,
    pub contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    pub generation_config: GenerationConfig,
}

#[derive(Serialize)]
pub struct SystemInstruction {
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
pub struct GenerationConfig {
    pub temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: u32,
    #[serde(rename = "topP")]
    pub top_p: f32,
    #[serde(rename = "topK")]
    pub top_k: u32,
}

#[derive(Serialize)]
pub struct GeminiContent {
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<InlineImageData>,
}

#[derive(Serialize)]
pub struct InlineImageData {
    pub mime_type: String,
    pub data: String,
}

#[derive(Deserialize)]
pub struct GeminiResponse {
    pub candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
pub struct GeminiCandidate {
    pub content: GeminiResponseContent,
}

#[derive(Deserialize)]
pub struct GeminiResponseContent {
    pub parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
pub struct GeminiResponsePart {
    pub text: String,
}
// Alternative: Keep using ChatResponse but with structured data
#[derive(Serialize)]
pub struct ChatResponse {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

// Or if you want to be more explicit:
#[derive(Serialize)]
pub struct ChatResponseStructured {
    pub dishes: Vec<Dish>,
}

#[derive(Serialize)]
pub struct Dish {
    #[serde(rename = "dishName")]
    pub dish_name: String,
    pub portions: std::collections::HashMap<String, String>,
    #[serde(rename = "preparationSteps")]
    pub preparation_steps: Vec<String>,
}

/// Response from /classify endpoint: dish identification + ingredients + nutrition
#[derive(Serialize, Deserialize, Debug)]
pub struct ClassifyResponse {
    #[serde(rename = "dishName")]
    pub dish_name: String,
    pub ingredients: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nutrients: Option<std::collections::HashMap<String, String>>,
}