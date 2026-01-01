import tensorflow as tf
import os

model_path = os.path.abspath("./models/blackblue/model.savedmodel")
print("Loading model from:", model_path)

model = tf.saved_model.load(model_path)
print("\n=== Available Signatures ===")
print(list(model.signatures.keys()))

# Get the default serving signature
if 'serving_default' in model.signatures:
    serving_default = model.signatures['serving_default']
    
    print("\n=== Serving Default Signature ===")
    print("\nInputs:")
    for input_name, input_spec in serving_default.structured_input_signature[1].items():
        print(f"  Name: {input_name}")
        print(f"  Shape: {input_spec.shape}")
        print(f"  Dtype: {input_spec.dtype}")
        print(f"  Tensor name: {serving_default.inputs[list(serving_default.inputs.keys())[0]].name}")
    
    print("\nOutputs:")
    for output_name, output_tensor in serving_default.structured_outputs.items():
        print(f"  Name: {output_name}")
        print(f"  Shape: {output_tensor.shape}")
        print(f"  Dtype: {output_tensor.dtype}")
        print(f"  Tensor name: {output_tensor.name}")

# Also print the concrete function details
print("\n=== All Operations in Graph ===")
concrete_func = model.signatures['serving_default']
for op in concrete_func.graph.get_operations():
    print(f"  {op.name} ({op.type})")