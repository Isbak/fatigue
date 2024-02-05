
use clap::{Arg, Command};
mod app_logic;
mod rainflow;
mod config;
mod stress;

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
        .after_help("Longer explanation to appear after the options when \
                     displaying the help information from --help or -h")
        .get_matches();
    if let Some(r) = matches.get_one::<String>("run") {
        let _ = app_logic::run(r);
    }
}

