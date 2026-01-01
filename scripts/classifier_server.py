import tensorflow as tf
import numpy as np
import sys
import json

class ClassifierServer:
    def __init__(self, model_path, labels_path):
        print("Loading model...", file=sys.stderr)
        self.model = tf.saved_model.load(model_path)
        self.infer = self.model.signatures["serving_default"]
        self.input_key = list(self.infer.structured_input_signature[1].keys())[0]
        
        with open(labels_path, 'r') as f:
            self.labels = [line.strip() for line in f.readlines()]
        
        print("Model loaded and ready!", file=sys.stderr)
    
    def classify(self, image_path):
        # Load and preprocess
        img = tf.io.read_file(image_path)
        img = tf.image.decode_image(img, channels=3, expand_animations=False)
        img = tf.image.resize(img, [224, 224])
        img = tf.cast(img, tf.float32) / 255.0
        img = tf.expand_dims(img, 0)
        
        # Run inference
        predictions = self.infer(**{self.input_key: img})
        output_key = list(predictions.keys())[0]
        output = predictions[output_key].numpy()[0]
        
        # Get top prediction
        predicted_idx = int(np.argmax(output))
        confidence = float(output[predicted_idx])
        predicted_label = self.labels[predicted_idx] if predicted_idx < len(self.labels) else f"Class_{predicted_idx}"
        
        return {
            "success": True,
            "predicted_label": predicted_label,
            "confidence": confidence
        }

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(json.dumps({"success": False, "error": "Usage: classifier_server.py <model_path> <labels_path>"}))
        sys.exit(1)
    
    model_path = sys.argv[1]
    labels_path = sys.argv[2]
    
    classifier = ClassifierServer(model_path, labels_path)
    
    # Read image paths from stdin and classify them
    print("READY", file=sys.stderr)
    for line in sys.stdin:
        image_path = line.strip()
        if not image_path:
            break
        
        try:
            result = classifier.classify(image_path)
            print(json.dumps(result), flush=True)
        except Exception as e:
            print(json.dumps({"success": False, "error": str(e)}), flush=True)