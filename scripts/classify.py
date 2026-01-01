import tensorflow as tf
import numpy as np
import sys
import json

def classify_image(image_path, model_path, labels_path):
    """
    Classify an image using the saved model.
    
    Args:
        image_path: Path to the image file
        model_path: Path to the SavedModel directory
        labels_path: Path to labels.txt file
    
    Returns:
        JSON string with classification results
    """
    try:
        # Load the model
        model = tf.saved_model.load(model_path)
        infer = model.signatures["serving_default"]
        
        # Load and preprocess the image
        img = tf.io.read_file(image_path)
        img = tf.image.decode_image(img, channels=3)
        img = tf.image.resize(img, [224, 224])
        img = tf.cast(img, tf.float32) / 255.0  # Normalize to [0, 1]
        img = tf.expand_dims(img, 0)  # Add batch dimension
        
        # Get input key name
        input_key = list(infer.structured_input_signature[1].keys())[0]
        
        # Run inference
        predictions = infer(**{input_key: img})
        
        # Get output
        output_key = list(predictions.keys())[0]
        output = predictions[output_key].numpy()[0]
        
        # Load labels
        with open(labels_path, 'r') as f:
            labels = [line.strip() for line in f.readlines()]
        
        # Get top prediction
        predicted_idx = np.argmax(output)
        confidence = float(output[predicted_idx])
        predicted_label = labels[predicted_idx] if predicted_idx < len(labels) else f"Class_{predicted_idx}"
        
        # Get top 3 predictions
        top_indices = np.argsort(output)[-3:][::-1]
        top_predictions = [
            {
                "label": labels[idx] if idx < len(labels) else f"Class_{idx}",
                "confidence": float(output[idx])
            }
            for idx in top_indices
        ]
        
        result = {
            "success": True,
            "predicted_label": predicted_label,
            "confidence": confidence,
            "top_predictions": top_predictions
        }
        
        return json.dumps(result)
        
    except Exception as e:
        error_result = {
            "success": False,
            "error": str(e)
        }
        return json.dumps(error_result)

if __name__ == "__main__":
    if len(sys.argv) != 4:
        print(json.dumps({
            "success": False,
            "error": "Usage: python classify.py <image_path> <model_path> <labels_path>"
        }))
        sys.exit(1)
    
    image_path = sys.argv[1]
    model_path = sys.argv[2]
    labels_path = sys.argv[3]
    
    result = classify_image(image_path, model_path, labels_path)
    print(result)