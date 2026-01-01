use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Prediction {
    pub label: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub success: bool,
    pub predicted_label: Option<String>,
    pub confidence: Option<f32>,
    pub top_predictions: Option<Vec<Prediction>>,
    pub error: Option<String>,
}

pub fn classify_image(
    image_path: &str,
) -> Result<ClassificationResult, Box<dyn std::error::Error + Send + Sync>> {
    let model_path = "models/blackblue/model.savedmodel";
    let labels_path = "models/blackblue/labels.txt";
    let script_path = "scripts/classify.py";
    
    // Use your conda environment Python
    let python_path = "/home/kaamaru/miniconda3/envs/basic/bin/python";
    
    // Check if files exist
    if !Path::new(image_path).exists() {
        return Err(format!("Image file not found: {}", image_path).into());
    }
    if !Path::new(model_path).exists() {
        return Err(format!("Model not found: {}", model_path).into());
    }
    if !Path::new(labels_path).exists() {
        return Err(format!("Labels file not found: {}", labels_path).into());
    }
    if !Path::new(script_path).exists() {
        return Err(format!("Python script not found: {}", script_path).into());
    }
    
    println!("Classifying image: {}", image_path);
    
    // Run the Python script
    let output = Command::new(python_path)
        .arg(script_path)
        .arg(image_path)
        .arg(model_path)
        .arg(labels_path)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Python script failed: {}", stderr).into());
    }
    
    // Parse the JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: ClassificationResult = serde_json::from_str(&stdout)?;
    
    Ok(result)
}

pub fn process_image(image_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match classify_image(image_path) {
        Ok(result) => {
            if result.success {
                println!("{:?}", result);
                println!("\n=== Classification Result ===");
                println!("Predicted: {} ({:.2}% confidence)", 
                    result.predicted_label.unwrap(),
                    result.confidence.unwrap() * 100.0
                );
                
                if let Some(top_preds) = result.top_predictions {
                    println!("\nTop 3 Predictions:");
                    for (i, pred) in top_preds.iter().enumerate() {
                        println!("  {}. {} - {:.2}%", 
                            i + 1, 
                            pred.label, 
                            pred.confidence * 100.0
                        );
                    }
                }
            } else {
                eprintln!("Classification failed: {}", result.error.unwrap());
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
    
    Ok(())
}