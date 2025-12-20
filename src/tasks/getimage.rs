use serde_json::Value;
use serpapi_search_rust::serp_api_search::SerpApiSearch;
use std::collections::HashMap;
use std::env;

pub async fn get_one_image(
    query: &str,
    
) -> Result<Option<String>, Box<dyn std::error::Error>> {

    
    let api_key = env::var("SERPAPI_API_KEY").expect("SERPAPI_API_KEY must be set");

    let mut params = HashMap::<String, String>::new();
    params.insert("q".into(), query.into());
    params.insert("engine".into(), "google_images".into());
    params.insert("ijn".into(), "0".into()); // first page

    let search = SerpApiSearch::google(params, api_key.to_string());
    let results: Value = search.json().await?;

    let images = results.get("images_results").and_then(|v| v.as_array());

    let first_image = images
        .and_then(|arr| arr.first())
        .and_then(|img| img.get("original").or_else(|| img.get("thumbnail")))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(first_image)
}
