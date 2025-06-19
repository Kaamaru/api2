use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub history: serde_json::Value,
    pub prompt: serde_json::Value,
    #[serde(rename = "promptlanguage")]
    pub prompt_language: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub response: String,
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
    pub text: String,
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
