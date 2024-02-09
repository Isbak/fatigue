
//! A module for the main application logic for the fatigue assessment tool
use clap::{Arg, Command};
mod app_logic;
mod rainflow;
mod config;
mod stress;
mod parser;

/// Main function for the application with a CLI interface using clap for argument parsing and subcommands
/// The main function should be used to parse the command line arguments and execute the application logic
/// based on the provided arguments. Following arguments are supported:
/// - `run`: The main argument to run the application followed by the path to the configuration file
/// - `mode`: The execution mode, which can be either `cloud` or `local`
/// 
/// # Example
/// ``` 
/// fatigue --run test.yaml
/// ```
fn main() {
    let matches = Command::new("Fatigue")
        .author("Kristoffer Isbak Thomsen, kristoffer.isbak@gmail.com")
        .version("0.1.0")
        .about("Safe and Fast Structural Fatigue Assessment as Code in Rust")
        .arg(
            Arg::new("run")
                .short('r') // Corrected: Use a character without the dash
                .long("run") // 'long' option names should not include dashes in the method call
                .help("Run the program") // Corrected: Moved the description to .help()
                .required(true) // Specify that this argument takes a value
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .value_name("MODE")
                .help("Sets the execution mode: cloud or local")
                .required(true),
        )
        .after_help("Longer explanation to appear after the options when \
                     displaying the help information from --help or -h")
        .get_matches();
    if let Some(r) = matches.get_one::<String>("run") {
        let _ = app_logic::run(r);
    }
}

