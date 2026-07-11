use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct PeopleContextItem {
    pub name: String,
    pub preference: Vec<String>,
    pub allergy: Vec<String>,
}

#[derive(Deserialize)]
pub struct ChatRequest {
    pub history: serde_json::Value,
    pub prompt: serde_json::Value,
    #[serde(rename = "promptlanguage")]
    pub prompt_language: String,
    #[serde(default)]
    pub userid: Option<String>,
    #[serde(default)]
    pub selected_people_ids: Option<Vec<String>>,
    #[serde(default)]
    pub people_context: Option<Vec<PeopleContextItem>>,
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
    /// Forces Gemini to return a specific MIME type (e.g. "application/json").
    #[serde(rename = "responseMimeType", skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,
    /// OpenAPI-subset schema that constrains the structured JSON output.
    #[serde(rename = "responseSchema", skip_serializing_if = "Option::is_none")]
    pub response_schema: Option<serde_json::Value>,
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

/// Query parameters for /search/recipes endpoint
#[derive(Deserialize, Debug)]
pub struct SearchQuery {
    pub name: String,
}

/// Raw per-item shape returned by Gemini Vision for batch inventory detection.
/// Matches the responseSchema sent in the request. Expiry is intentionally NOT
/// requested from the model — it is computed server-side from shelf_life_days().
#[derive(Deserialize, Debug, Clone)]
pub struct GeminiBatchItem {
    #[serde(default)]
    pub name_th: String,
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub estimated_quantity: String,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub confidence: f32,
}

/// A single draft inventory item returned to the frontend review table.
/// Not persisted to Firestore — the user confirms/edits first.
#[derive(Serialize, Debug, Clone)]
pub struct BatchDetectItem {
    pub name_th: String,
    pub name_en: String,
    pub category: String,
    pub estimated_quantity: String,
    pub unit: String,
    pub confidence: f32,
    /// Server-computed expiry (YYYY-MM-DD) = today + shelf_life_days(category).
    pub suggested_expiry: String,
    /// 0-based index of the uploaded image this item was detected in.
    pub source_image_index: usize,
}

/// Response for POST /batch-detect: the draft list the user will review.
#[derive(Serialize, Debug)]
pub struct BatchDetectResponse {
    pub items: Vec<BatchDetectItem>,
    pub image_count: usize,
}

/// JSON body for explicit recipe generation endpoint
#[derive(Deserialize, Debug)]
pub struct GenerateRecipeRequest {
    pub name: String,
    #[serde(default)]
    pub userid: Option<String>,
    #[serde(default)]
    pub list_id: Option<String>,
    #[serde(default)]
    pub save_to_grocery: Option<bool>,
}

/// JSON body for adding an existing recipe into a grocery list
#[derive(Deserialize, Debug)]
pub struct AddRecipeToListRequest {
    pub userid: String,
    pub list_id: String,
}

/// JSON body for creating a new grocery list from recipe information
#[derive(Deserialize, Debug)]
pub struct CreateRecipeListFromInfoRequest {
    pub userid: String,
    pub recipe_name: String,
    pub ingredients: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub list_name: Option<String>,
    #[serde(default)]
    pub nutrition: Option<std::collections::HashMap<String, String>>,
}

/// Cache entry for fuzzy search index
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DishName {
    pub recipe_id: String,
    pub normalized: String,
    /// Alternative names/spellings (pre-normalized) for better matching
    /// e.g., transliterations, common misspellings. Limited to 5 per dish.
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// Recipe data for frontend display
#[derive(Serialize, Debug, Clone)]
pub struct RecipeData {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingredients: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nutrition: Option<std::collections::HashMap<String, String>>,
}

/// Match quality information (debug + UX hints)
#[derive(Serialize, Debug, Clone)]
pub struct MatchInfo {
    pub matched_from: String,
    pub score: f64,
    /// Confidence level for UX: "exact", "high", "low"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
}

/// Frontend-friendly API response for /search/recipes
#[derive(Serialize, Debug)]
pub struct SearchApiResponse {
    pub status: String, // "found", "not_found", "generating", "failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<RecipeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_info: Option<MatchInfo>,
    /// For status="generating": retry after this many seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<u32>,
}

/// Response from /search/recipes endpoint (legacy, for debug)
#[derive(Serialize, Debug)]
pub struct SearchRecipeResponse {
    #[serde(rename = "dishName")]
    pub dish_name: String,
    pub ingredients: std::collections::HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calories: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nutrients: Option<std::collections::HashMap<String, String>>,
    /// Which normalized string or alias was matched
    #[serde(rename = "matchedFrom")]
    pub matched_from: Option<String>,
    /// Debug info: fuzzy match score (0.0-1.0) for tuning
    pub score: Option<f64>,
}