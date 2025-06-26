use crate::firebase::GEMINI_API_KEY;
use crate::structures::models::{
    ChatRequest, ChatResponse, GeminiContent, GeminiPart, GeminiRequest, GeminiResponse,
    GenerationConfig, SystemInstruction,
};
use axum::{extract::Json, http::StatusCode, response::Json as ResponseJson};
use reqwest::Client;
use serde_json::Value;

pub async fn chat_handler(
    Json(payload): Json<ChatRequest>,
) -> Result<ResponseJson<Value>, StatusCode> {
    // Return Value directly
    let api_key = GEMINI_API_KEY;

    // Extract prompt text
    let prompt_text = match &payload.prompt {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(obj) => obj
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    };

    let system_instruction_text = format!(
        "You are a highly creative, reliable, and friendly cooking assistant named NekoChef.\n\nYour task is to help users discover delicious meals they can cook using the ingredients they provide.\n\nGiven the following ingredients: {}\n\nSuggest exactly three distinct dishes. Each dish must:\n- Have a clear and appealing \"dishName\".\n- Include a \"portions\" object with ingredient amounts using appropriate units (grams, milliliters, pieces, tablespoons, teaspoons, cups, etc.).\n- Include a \"preparationSteps\" array, where each item is a single clear step of the recipe.\n- Use common cooking knowledge to make the best use of the given ingredients, even if not all are used in each dish.\n- Be imaginative, yet realistic to prepare in a regular kitchen.\n- Provide realistic portion sizes for 2-3 servings.\n\nRespond strictly as a JSON object with the following structure:\n\n{{\n  \"dishes\": [\n    {{\n      \"dishName\": \"Dish Title\",\n      \"portions\": {{\n        \"ingredient1\": \"amount with unit\",\n        \"ingredient2\": \"amount with unit\",\n        \"oil\": \"2 tablespoons\",\n        \"salt\": \"1 teaspoon\"\n      }},\n      \"preparationSteps\": [\n        \"Step 1...\",\n        \"Step 2...\",\n        \"...etc\"\n      ]\n    }},\n    ... 2 more dishes\n  ]\n}}\n\nPlease provide the entire response in the {} language.\nDo **not** include any extra text, commentary, or formatting outside of the JSON object.",
        prompt_text, payload.prompt_language
    );

    let content_text = format!(
        "Ingredients: {}\nLanguage: {}",
        prompt_text, payload.prompt_language
    );

    let gemini_request = GeminiRequest {
        system_instruction: Some(SystemInstruction {
            parts: vec![GeminiPart {
                text: system_instruction_text,
            }],
        }),
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: content_text }],
        }],
        generation_config: GenerationConfig {
            temperature: 1.0,
            max_output_tokens: 800,
            top_p: 0.8,
            top_k: 10,
        },
    };

    let client = Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}",
        api_key
    );

    println!("Sending request to: {}", url);
    println!(
        "Request body: {}",
        serde_json::to_string_pretty(&gemini_request).unwrap()
    );

    let response = client
        .post(&url)
        .json(&gemini_request)
        .send()
        .await
        .map_err(|e| {
            println!("Request error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    println!("Response status: {}", response.status());

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        println!("API error response: {}", error_text);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let gemini_response: GeminiResponse = response.json().await.map_err(|e| {
        println!("JSON parse error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "No response".to_string());

    println!("Raw Gemini response: {}", response_text);
    println!(
        "Response starts with ```json: {}",
        response_text.starts_with("```json")
    );
    println!(
        "Response starts with ```: {}",
        response_text.starts_with("```")
    );

    // Clean up the response - remove markdown code blocks
    let cleaned_response = {
        let mut text = response_text.trim().to_string();

        // Remove code block markers
        if text.starts_with("```json") {
            text = text.strip_prefix("```json").unwrap_or(&text).to_string();
        } else if text.starts_with("```") {
            text = text.strip_prefix("```").unwrap_or(&text).to_string();
        }

        if text.ends_with("```") {
            text = text.strip_suffix("```").unwrap_or(&text).to_string();
        }

        // Trim any remaining whitespace
        text.trim().to_string()
    };

    println!("Cleaned response: {}", cleaned_response);
    println!("Attempting to parse JSON...");

    // Parse the JSON response
    let parsed_json: Value = serde_json::from_str(&cleaned_response).map_err(|e| {
        println!("Failed to parse Gemini response as JSON: {}", e);
        println!("Raw response: {}", response_text);
        println!("Cleaned response: {}", cleaned_response);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Pretty print to terminal (optional - you can remove this if not needed)
    if let Some(dishes) = parsed_json.get("dishes").and_then(|d| d.as_array()) {
        println!("\n🍳 NekoChef's Recipe Suggestions:");
        println!("================================");

        for (i, dish) in dishes.iter().enumerate() {
            if let Some(name) = dish.get("dishName").and_then(|n| n.as_str()) {
                println!("\n{}. 🍽️  {}", i + 1, name);

                // Print portions if available
                if let Some(portions) = dish.get("portions").and_then(|p| p.as_object()) {
                    println!("   📏 Ingredients:");
                    for (ingredient, amount) in portions {
                        if let Some(amount_str) = amount.as_str() {
                            println!("   - {}: {}", ingredient, amount_str);
                        }
                    }
                }

                // Print steps
                if let Some(steps) = dish.get("preparationSteps").and_then(|s| s.as_array()) {
                    println!("   👨‍🍳 Steps:");
                    for (step_num, step) in steps.iter().enumerate() {
                        if let Some(step_text) = step.as_str() {
                            println!("   {}. {}", step_num + 1, step_text);
                        }
                    }
                }
            }
        }
        println!("\n================================");
    }

    // Return the parsed JSON directly
    Ok(ResponseJson(parsed_json))
}
pub async fn call_gemini_api(gemini_request: &GeminiRequest) -> Result<String, reqwest::Error> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .unwrap_or_else(|_| "AIzaSyCycTkqXarUfS1UyIaMi60NC0UqqvJ6BJ4".to_string());

    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}", api_key);

    let client = Client::new();

    let response = client.post(&url).json(gemini_request).send().await?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        eprintln!("Gemini API error: {}", err_text);
        return Ok("API Error".to_string());
    }

    let gemini_response: GeminiResponse = response.json().await?;

    let response_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "No response".to_string());

    // Remove markdown code fences if present
    let cleaned = if response_text.starts_with("```json") {
        response_text
            .strip_prefix("```json\n")
            .unwrap_or(&response_text)
            .strip_suffix("\n```")
            .unwrap_or(&response_text)
            .to_string()
    } else {
        response_text
    };

    Ok(cleaned)
}
