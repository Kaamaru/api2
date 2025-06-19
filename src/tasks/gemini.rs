use crate::structures::models::{
    ChatRequest, ChatResponse, GeminiContent, GeminiPart, GeminiRequest, GeminiResponse,
    GenerationConfig, SystemInstruction,
};
use axum::{extract::Json, http::StatusCode, response::Json as ResponseJson};
use reqwest::Client;
use serde_json::Value;

pub async fn chat_handler(
    Json(payload): Json<ChatRequest>,
) -> Result<ResponseJson<ChatResponse>, StatusCode> {
    let api_key = "AIzaSyCycTkqXarUfS1UyIaMi60NC0UqqvJ6BJ4"; // Replace with your actual API key

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
        "You are a highly creative, reliable, and friendly cooking assistant named NekoChef.\n\nYour task is to help users discover delicious meals they can cook using the ingredients they provide.\n\nGiven the following ingredients: {}\n\nSuggest exactly three distinct dishes. Each dish must:\n- Have a clear and appealing \"dishName\".\n- Include a \"preparationSteps\" array, where each item is a single clear step of the recipe.\n- Use common cooking knowledge to make the best use of the given ingredients, even if not all are used in each dish.\n- Be imaginative, yet realistic to prepare in a regular kitchen.\n\nRespond strictly as a JSON object with the following structure:\n\n{{\n  \"dishes\": [\n    {{\n      \"dishName\": \"Dish Title\",\n      \"preparationSteps\": [\n        \"Step 1...\",\n        \"Step 2...\",\n        \"...etc\"\n      ]\n    }},\n    ... 2 more dishes\n  ]\n}}\n\nPlease provide the entire response in the {} language.\nDo **not** include any extra text, commentary, or formatting outside of the JSON object.",
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

    // Clean up the response - remove markdown code blocks
    let cleaned_response = if response_text.starts_with("```json") {
        response_text
            .strip_prefix("```json\n")
            .unwrap_or(&response_text)
            .strip_suffix("\n```")
            .unwrap_or(&response_text)
            .to_string()
    } else {
        response_text.clone()
    };

    // Parse and extract dishes for development use
    let mut dishes_vec: Vec<(String, Vec<String>)> = Vec::new();
    let mut dish_names: Vec<String> = Vec::new();
    let mut all_steps: Vec<Vec<String>> = Vec::new();

    // Pretty print to terminal
    if let Ok(parsed_json) = serde_json::from_str::<serde_json::Value>(&cleaned_response) {
        println!("\n🍳 NekoChef's Recipe Suggestions:");
        println!("================================");

        if let Some(dishes) = parsed_json.get("dishes").and_then(|d| d.as_array()) {
            for (i, dish) in dishes.iter().enumerate() {
                if let (Some(name), Some(steps)) = (
                    dish.get("dishName").and_then(|n| n.as_str()),
                    dish.get("preparationSteps").and_then(|s| s.as_array()),
                ) {
                    // Store dish name
                    let dish_name = name.to_string();
                    dish_names.push(dish_name.clone());

                    // Store steps
                    let mut steps_vec: Vec<String> = Vec::new();
                    for step in steps.iter() {
                        if let Some(step_text) = step.as_str() {
                            steps_vec.push(step_text.to_string());
                        }
                    }
                    all_steps.push(steps_vec.clone());

                    // Store as tuple (name, steps)
                    dishes_vec.push((dish_name.clone(), steps_vec));

                    // Print to terminal
                    println!("\n{}. 🍽️  {}", i + 1, dish_name);
                    println!("   Steps:");
                    for (step_num, step) in steps.iter().enumerate() {
                        if let Some(step_text) = step.as_str() {
                            println!("   {}. {}", step_num + 1, step_text);
                        }
                    }
                }
            }
        }
        println!("\n================================");

        // Debug print for development
        println!("📊 Extracted data for development:");
        println!("Total dishes: {}", dishes_vec.len());
        println!("Dish names: {:?}", dish_names);
        println!(
            "Steps count per dish: {:?}",
            all_steps.iter().map(|s| s.len()).collect::<Vec<_>>()
        );
        println!();
    }

    Ok(ResponseJson(ChatResponse {
        response: cleaned_response,
    }))
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
