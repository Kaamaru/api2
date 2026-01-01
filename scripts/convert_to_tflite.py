import tensorflow as tf

# Load model
model = tf.saved_model.load("models/blackblue/model.savedmodel")
concrete_func = model.signatures["serving_default"]

# Convert to TFLite with optimizations
converter = tf.lite.TFLiteConverter.from_concrete_functions([concrete_func])
converter.optimizations = [tf.lite.Optimize.DEFAULT]  # Quantization
converter.target_spec.supported_ops = [tf.lite.OpsSet.TFLITE_BUILTINS]

tflite_model = converter.convert()

# Save
with open("models/blackblue/model.tflite", "wb") as f:
    f.write(tflite_model)

print("TFLite model saved! Size reduced significantly.")