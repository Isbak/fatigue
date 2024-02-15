//! A module for the main application logic for the fatigue assessment tool
#[cfg(any(feature = "cli", feature = "wasm"))]
mod rainflow;
#[cfg(feature = "cli")]
mod app_logic;
#[cfg(feature = "cli")]
mod config;
#[cfg(feature = "cli")]
mod stress;
#[cfg(feature = "cli")]
mod interpolate;
#[cfg(feature = "cli")]
mod material;
#[cfg(feature = "cli")]
mod timeseries;
#[cfg(feature = "cli")]
use clap::{Arg, Command};

// Code specific to the CLI build
#[cfg(feature = "cli")]
mod cli {
    // CLI-specific code here
    // You can define your CLI interactions here and call them from the main function if the CLI feature is enabled
}

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
        app_logic::run(r);
    }

    // Additional CLI logic would be here
}

#[cfg(not(feature = "cli"))]
fn main() {
    println!("This binary was not compiled with CLI support.");
}
