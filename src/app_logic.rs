//! A module for the main application logic for the fatigue assessment tool
use crate::config::load_config;
use crate::parser::parse_input;
pub use crate::stress::read_stress_tensors_from_file;

pub fn run(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running with configuration: {}", config_path);
    let conf = load_config(config_path)?;
    let res = parse_input(&conf.timeseries);
    for inter in conf.timeseries.interpolations.iter() {
        for point in inter.points.iter() {
            let stress = read_stress_tensors_from_file(inter, &point);
            println!("Stress tensors: {:?}", stress);
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
