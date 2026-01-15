use tract_onnx::prelude::*;


pub const MODEL_PATH: &str = "models/mobilenetv2-7.onnx";
pub fn tract_test() {
    let model = RunnableOnnxModel::create_tract_model(MODEL_PATH);

    let image_list = vec![
        create_input_image("assets/tomato.jpg"),
        create_input_image("assets/lettuce.jpg"),
    ];
    println!("Running model on test images...");
    if let Ok(result) = model.run_model(image_list) {
        println!("{:?}", result);
    }else {
        println!("Failed to run model");
    }
}


pub struct RunnableOnnxModel {
    model: SimplePlan<
        TypedFact,
        Box<dyn TypedOp>,
        tract_onnx::prelude::Graph<TypedFact, Box<dyn TypedOp>>,
    >,
    model_path: String,
}

impl RunnableOnnxModel {
    pub fn create_tract_model(model_path: &str) -> Self {
        let model = tract_onnx::onnx()
            // load the model
            .model_for_path(model_path)
            .unwrap()
            // optimize the model
            .into_optimized()
            .unwrap()
            // make the model runnable and fix its inputs and outputs
            .into_runnable()
            .unwrap();

        Self {
            model,
            model_path: model_path.to_string(),
        }
    }

    pub fn run_model(&self, image_list: Vec<Tensor>) -> Result<Vec<(f32, isize)>, anyhow::Error> {
        let mut result_list: Vec<(f32, isize)> = vec![];

        for image in image_list {
            let results = self.model.run(tvec!(image.into()))?;

            // find and display the max value with its index
            let best = results[0]
                .to_array_view::<f32>()?
                .iter()
                .cloned()
                .zip(2..)
                .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            // println!("result: {:?}", best);

            result_list.push(best.unwrap());
        }

        Ok(result_list)
    }
}

pub fn create_input_image(image_path: &str) -> Tensor {
    let image = image::open(image_path).unwrap().to_rgb8();
    let resized =
        image::imageops::resize(&image, 224, 224, ::image::imageops::FilterType::Triangle);
    let resized_image: Tensor =
        tract_ndarray::Array4::from_shape_fn((1, 3, 224, 224), |(_, c, y, x)| {
            let mean = [0.485, 0.456, 0.406][c];
            let std = [0.229, 0.224, 0.225][c];
            (resized[(x as _, y as _)][c] as f32 / 255.0 - mean) / std
        })
        .into();
    resized_image
}
