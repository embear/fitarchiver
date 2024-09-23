//! # FIT file archiver
//!
//! `fitarchiver` is a tool to copy or move FIT files based on information contained in the file.

use std::process::ExitCode;

mod fitarchiver;

mod my_module {
    // your code here
}

fn main() -> ExitCode {
    match fitarchiver::process_files(&fitarchiver::parse_arguments(None)) {
        Ok(val) => println!("{}", val),
        Err(val) => eprintln!("ERROR: {}", val),
    };

    ExitCode::SUCCESS
}
