//! A module for the main application logic for the fatigue assessment tool
use crate::config::load_config;
pub use crate::stress::read_stress_tensors_from_file;
use std::path::PathBuf;

pub fn run(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running with configuration: {}", config_path);
    let conf = load_config(config_path)?;
    let res = conf.timeseries.parse_input();
    for inter in conf.timeseries.interpolations.iter() {
        for point in inter.points.iter() {
            if let Some(ref file_name) = point.file{
                let path = PathBuf::from(&inter.path).join(file_name); // Correctly constructs the path
                let tensors = read_stress_tensors_from_file(&path)?; // Assuming the function accepts a `&Path`
                println!("Stress tensors: {:?}", tensors);
            }
        }
    }

    println!("Results: {:?}", res);
    if let Err(err) = conf.validate() {
        // Handle the error here
        println!("Validation error: {:?}", err);
    }
    println!("Configuration: {:?}", conf);
    // Here, you would add the logic to load the configuration from the specified path,
    // and then execute the functionality of your application based on that configuration.

    // Return Ok(()) to indicate success

    Ok(())
}
