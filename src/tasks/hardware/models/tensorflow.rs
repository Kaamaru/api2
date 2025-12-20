use tensorflow::{Graph, SessionOptions, SessionRunArgs, SavedModelBundle, Tensor};

fn main() {
    // Load SavedModel
    let mut graph = Graph::new();
    let bundle = SavedModelBundle::load(
        &SessionOptions::new(),
        &["serve"],
        &mut graph,
        "model/"
    ).expect("failed to load model");

    let session = &bundle.session;

    // --- PREPARE INPUT TENSOR ---
    // Example: 224x224 RGB image normalized 0..1
    let img = vec![0.0_f32; 1 * 224 * 224 * 3];
    let input_tensor = Tensor::<f32>::new(&[1, 224, 224, 3])
        .with_values(&img)
        .unwrap();

    // --- SETUP RUN ARGUMENTS ---
    let mut args = SessionRunArgs::new();

    // CHANGE THESE TWO NAMES:
    let input_op = graph.operation_by_name_required("serving_default_input_1").unwrap();
    let output_op = graph.operation_by_name_required("StatefulPartitionedCall").unwrap();

    args.add_feed(&input_op, 0, &input_tensor);
    let output_token = args.request_fetch(&output_op, 0);

    // --- RUN INFERENCE ---
    session.run(&mut args).unwrap();

    let output: Tensor<f32> = args.fetch(output_token).unwrap();

    println!("Raw output: {:?}", output);
}
