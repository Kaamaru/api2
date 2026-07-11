//! Batch photo detection for the Food Inventory.
//!
//! Accepts one or more uploaded images (multipart) and sends each to Gemini
//! Vision with a responseSchema-constrained request that forces structured JSON
//! output. Follows the same conventions as `gemini::classify_image_handler`
//! (client setup, `x-goog-api-key` auth, image preprocessing, error handling).
//!
//! The model is NOT allowed to guess expiry dates: `suggested_expiry` is computed
//! server-side from `shelf_life_days(category)` added to today's date. The draft
//! list is returned to the frontend WITHOUT writing to Firestore.

use crate::firebase::GeminiConfig;
use crate::structures::models::{
    BatchDetectItem, BatchDetectResponse, GeminiBatchItem, GeminiContent, GeminiPart,
    GeminiRequest, GeminiResponse, GenerationConfig, InlineImageData, SystemInstruction,
};
use crate::tasks::gemini::{base64_encode, preprocess_image_for_gemini};
use axum::{extract::Multipart, http::StatusCode, response::Json as ResponseJson};
use chrono::Local;
use reqwest::Client;
use serde_json::{json, Value};

/// Reject obviously abusive uploads; each image is a separate Gemini call.
const MAX_IMAGES: usize = 12;

/// The exact category strings the existing Foods inventory UI understands
/// (see `FOOD_CATEGORIES` in the frontend `Foods.tsx`). "Eggs" is the single
/// genuinely-new bucket added for this feature. "Unspecified" is intentionally
/// omitted from what the model may choose — it is a UI-only placeholder; the
/// model falls back to "Other" when unsure.
const MODEL_CATEGORIES: &[&str] = &[
    "Dairy",
    "Eggs",
    "Meat",
    "Poultry",
    "Seafood",
    "Vegetables",
    "Fruits",
    "Grains",
    "Legumes",
    "Nuts",
    "Herbs",
    "Condiments",
    "Oils",
    "Bakery",
    "Snacks",
    "Beverages",
    "Frozen",
    "Canned",
    "ReadyToEat",
    "Desserts",
    "Other",
];

/// Static shelf-life lookup keyed off the existing category strings. Values are
/// seeded from the day-counts already used by the frontend Foods `expiry` map,
/// with "Eggs" added (~28 days, matching `ExpiryList.json` eggs_refrigerated).
pub fn shelf_life_days(category: &str) -> i64 {
    match category {
        "Dairy" => 7,
        "Eggs" => 28,
        "Meat" => 3,
        "Poultry" => 2,
        "Seafood" => 1,
        "Vegetables" => 3,
        "Fruits" => 5,
        "Grains" => 30,
        "Legumes" => 14,
        "Nuts" => 60,
        "Herbs" => 7,
        "Condiments" => 90,
        "Oils" => 180,
        "Bakery" => 5,
        "Snacks" => 30,
        "Beverages" => 14,
        "Frozen" => 90,
        "Canned" => 365,
        "ReadyToEat" => 3,
        "Desserts" => 7,
        // "Unspecified", "Other", and anything unexpected use the generic default.
        _ => 7,
    }
}

/// Normalize a model-provided category to one of the known category strings,
/// case-insensitively. Anything unrecognized becomes "Other".
fn normalize_category(raw: &str) -> String {
    let trimmed = raw.trim();
    MODEL_CATEGORIES
        .iter()
        .find(|c| c.eq_ignore_ascii_case(trimmed))
        .map(|c| c.to_string())
        .unwrap_or_else(|| "Other".to_string())
}

/// Build the responseSchema (OpenAPI subset) that forces Gemini to return an
/// array of inventory items with the category constrained to our enum.
fn batch_response_schema() -> Value {
    let category_enum: Vec<&str> = MODEL_CATEGORIES.to_vec();
    json!({
        "type": "ARRAY",
        "items": {
            "type": "OBJECT",
            "properties": {
                "name_th": { "type": "STRING" },
                "name_en": { "type": "STRING" },
                "category": { "type": "STRING", "format": "enum", "enum": category_enum },
                "estimated_quantity": { "type": "STRING" },
                "unit": { "type": "STRING" },
                "confidence": { "type": "NUMBER" }
            },
            "required": [
                "name_th", "name_en", "category",
                "estimated_quantity", "unit", "confidence"
            ]
        }
    })
}

/// Send a single preprocessed image to Gemini Vision and return the raw detected
/// items. Uses responseSchema so the response is guaranteed structured JSON.
async fn detect_items_in_image(
    client: &Client,
    api_key: &str,
    url: &str,
    base64_image: String,
    mime_type: String,
) -> Result<Vec<GeminiBatchItem>, StatusCode> {
    let system_prompt = "You are a food inventory assistant. Look at the photo and \
identify every distinct food or grocery item you can see. For each item provide: \
name_th (Thai name), name_en (English name), the best-fitting category from the \
allowed list, an estimated_quantity (a short string such as \"2\" or \"500\"), a \
unit (e.g. pcs, g, kg, ml, l, pack, bottle), and a confidence between 0 and 1. \
Do NOT include expiry dates. If no food is visible, return an empty array. \
Respond ONLY with the JSON array defined by the schema.";

    let gemini_request = GeminiRequest {
        system_instruction: Some(SystemInstruction {
            parts: vec![GeminiPart {
                text: Some(system_prompt.to_string()),
                inline_data: None,
            }],
        }),
        contents: vec![GeminiContent {
            parts: vec![
                GeminiPart {
                    text: Some("Detect the food items in this image.".to_string()),
                    inline_data: None,
                },
                GeminiPart {
                    text: None,
                    inline_data: Some(InlineImageData {
                        mime_type,
                        data: base64_image,
                    }),
                },
            ],
        }],
        generation_config: GenerationConfig {
            temperature: 0.2,
            max_output_tokens: 4000,
            top_p: 0.8,
            top_k: 40,
            response_mime_type: Some("application/json".to_string()),
            response_schema: Some(batch_response_schema()),
        },
    };

    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&gemini_request)
        .send()
        .await
        .map_err(|e| {
            eprintln!("batch-detect: Gemini request failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        eprintln!("batch-detect: Gemini error response: {}", err_text);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let gemini_response: GeminiResponse = response.json().await.map_err(|e| {
        eprintln!("batch-detect: failed to parse Gemini envelope: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "[]".to_string());

    // With responseSchema the output is already raw JSON, but strip fences
    // defensively to match the tolerance of the existing classify handler.
    let cleaned = {
        let mut text = response_text.trim().to_string();
        if text.starts_with("```json") {
            text = text.strip_prefix("```json").unwrap_or(&text).to_string();
        } else if text.starts_with("```") {
            text = text.strip_prefix("```").unwrap_or(&text).to_string();
        }
        if text.ends_with("```") {
            text = text.strip_suffix("```").unwrap_or(&text).to_string();
        }
        text.trim().to_string()
    };

    let items: Vec<GeminiBatchItem> = serde_json::from_str(&cleaned).map_err(|e| {
        eprintln!("batch-detect: failed to parse items array: {}\n{}", e, cleaned);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(items)
}

/// POST /batch-detect — multipart form with one or more image fields.
/// Returns a draft list for the review table; nothing is written to Firestore.
pub async fn batch_detect_handler(
    mut multipart: Multipart,
) -> Result<ResponseJson<BatchDetectResponse>, StatusCode> {
    // Collect every uploaded image (any field carrying an image content-type).
    let mut images: Vec<(Vec<u8>, String)> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let mut mime_type = field.content_type().unwrap_or("").to_string();
        if mime_type.is_empty() {
            mime_type = "image/jpeg".to_string();
        }

        if !mime_type.contains("image/png") && !mime_type.contains("image/jpeg") {
            // Ignore non-image fields rather than failing the whole batch.
            continue;
        }

        let bytes = field
            .bytes()
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .to_vec();

        if bytes.is_empty() {
            continue;
        }

        images.push((bytes, mime_type));

        if images.len() >= MAX_IMAGES {
            break;
        }
    }

    if images.is_empty() {
        eprintln!("batch-detect: no images provided in multipart form");
        return Err(StatusCode::BAD_REQUEST);
    }

    let api_key = GeminiConfig::from_env().api_key;
    let url = GeminiConfig::from_env().base_url;
    let client = Client::new();

    let today = Local::now().date_naive();
    let mut all_items: Vec<BatchDetectItem> = Vec::new();

    for (index, (image_bytes, mime_type)) in images.iter().enumerate() {
        let (prepared_bytes, prepared_mime) = preprocess_image_for_gemini(image_bytes, mime_type);
        let base64_image = base64_encode(&prepared_bytes);

        let detected =
            detect_items_in_image(&client, &api_key, &url, base64_image, prepared_mime).await?;

        for raw in detected {
            let category = normalize_category(&raw.category);
            let expiry_date = today + chrono::Duration::days(shelf_life_days(&category));
            let confidence = raw.confidence.clamp(0.0, 1.0);

            all_items.push(BatchDetectItem {
                name_th: raw.name_th,
                name_en: raw.name_en,
                category,
                estimated_quantity: raw.estimated_quantity,
                unit: raw.unit,
                confidence,
                suggested_expiry: expiry_date.format("%Y-%m-%d").to_string(),
                source_image_index: index,
            });
        }
    }

    let image_count = images.len();
    eprintln!(
        "batch-detect: {} image(s) -> {} draft item(s)",
        image_count,
        all_items.len()
    );

    Ok(ResponseJson(BatchDetectResponse {
        items: all_items,
        image_count,
    }))
}
