import tensorflow as tf
import shutil
import os

# Load the model
model = tf.saved_model.load("models/blackblue/model.savedmodel")

# Get the concrete function
concrete_func = model.signatures["serving_default"]

# Create a new model directory
output_path = "models/blackblue/model_converted"
if os.path.exists(output_path):
    shutil.rmtree(output_path)

# Save with explicit options for compatibility
tf.saved_model.save(
    model,
    output_path,
    signatures={"serving_default": concrete_func}
)

print(f"Model converted and saved to: {output_path}")

# Verify the conversion
loaded = tf.saved_model.load(output_path)
print("Conversion verified successfully!")
print("Input:", list(loaded.signatures["serving_default"].structured_input_signature[1].keys()))
print("Output:", list(loaded.signatures["serving_default"].structured_outputs.keys()))