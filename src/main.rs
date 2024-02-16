//! A module for the main application logic for the fatigue assessment tool
#[cfg(any(feature = "cli", feature = "wasm"))]
pub mod rainflow;
#[cfg(feature = "cli")]
mod app_logic;
#[cfg(feature = "cli")]
pub mod config;
#[cfg(feature = "cli")]
pub mod stress;
#[cfg(any(feature = "cli", feature = "wasm"))]
pub mod interpolate;
pub use interpolate::{InterpolationStrategy, Linear, NDInterpolation};

#[cfg(feature = "cli")]
pub mod material;
#[cfg(feature = "cli")]
pub mod timeseries;
#[cfg(feature = "cli")]
use clap::{Arg, Command};

#[cfg(feature = "cli")]
fn main() {
    let matches = Command::new("Fatigue")
        .author("Kristoffer Isbak Thomsen, kristoffer.isbak@gmail.com")
        .version("0.1.0")
        .about("Safe and Fast Structural Fatigue Assessment as Code in Rust")
        .arg(
            Arg::new("run")
                .short('r')
                .long("run")
                .required(false)
                .help("Run the program with the specified configuration file")
        )
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .required(false)
                .help("Sets the execution mode: cloud or local")
        )
        .arg(
            Arg::new("rainflow")
                .short('a')
                .long("rainflow")
                .required(false)
                .help("Perform rainflow counting on the input data")
        )        
        .after_help("Longer explanation to appear after the options when \
                     displaying the help information from --help or -h")
        .get_matches();

    // Match the commands and execute the appropriate functionality
    if let Some(r) = matches.get_one::<String>("run") {
        if let Err(e) = app_logic::run(r) {
            println!("Error running app logic: {:?}", e);
            // You could return an error here, or take other corrective actions as needed.
        }
    }

    // Additional CLI logic would be here
}

