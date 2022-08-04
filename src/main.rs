use std::fs::File;
use std::panic;
use std::process::ExitCode;

fn main() -> ExitCode {
    println!(
        "Parsing FIT files using Profile version: {}",
        fitparser::profile::VERSION
    );

    // open FIT file
    let mut fp = match File::open("/tmp/activity.fit") {
        Ok(fp) => fp,
        Err(err) => {
            println!("Error: {}", err);
            return ExitCode::FAILURE;
        }
    };

    // parse fit file to data structure
    let alldata = match fitparser::from_reader(&mut fp) {
        Ok(alldata) => alldata,
        Err(err) => {
            println!("Error: {}", err);
            return ExitCode::FAILURE;
        }
    };

    // iterate over all data elements
    for data in alldata {
        match data.kind() {
            fitparser::profile::field_types::MesgNum::FileId => {
                for field in data.fields() {
                    match field.name() {
                        "time_created" => {
                            match &field.value() {
                                fitparser::Value::Timestamp(val) => {
                                    // store val somewhere else
                                    println!("file: {}", val.format("%Y%m%dT%H%M%SZ"));
                                    println!("directory: {}", val.format("%Y/%m"));
                                }
                                &_ => panic!("unexpected value in enum fitparser::Value"),
                            }
                        }
                        "type" => {
                            match &field.value() {
                                fitparser::Value::String(val) => {
                                    // actually here an error message should be generated because
                                    // only "type" == "activity" makes sense
                                    println!("type: {}", val);
                                }
                                &_ => panic!("unexpected value in enum fitparser::Value"),
                            }
                        }
                        &_ => (), // ignore all other values
                    }
                }
            }
            fitparser::profile::field_types::MesgNum::Sport => {
                for field in data.fields() {
                    match field.name() {
                        "sport" => {
                            match &field.value() {
                                fitparser::Value::String(val) => {
                                    // store val somewhere else
                                    println!("type: {}", val);
                                }
                                &_ => panic!("unexpected value in enum fitparser::Value"),
                            }
                        }
                        &_ => (), // ignore all other values
                    }
                }
            }
            _ => (), // ignore all other values
        }
    }

    return ExitCode::SUCCESS;
}
