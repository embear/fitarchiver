use chrono::{DateTime, Local, TimeZone};
use std::fs::File;
use std::process::ExitCode;

struct ActivityData {
    sport: String,
    timestamp: DateTime<Local>,
}

fn parse_fit_file(filename: String) -> Result<ActivityData, String> {
    let mut activity_data = ActivityData {
        sport: String::from("unknown"),
        timestamp: chrono::Local.ymd(1970, 1, 1).and_hms(0, 0, 0),
    };

    // open FIT file
    let mut fp = match File::open(filename) {
        Ok(fp) => fp,
        Err(_err) => return Err(String::from("unable to open FIT file")),
    };

    // parse FIT file to data structure
    let parsed_data = match fitparser::from_reader(&mut fp) {
        Ok(parsed_data) => parsed_data,
        Err(_err) => return Err(String::from("unable to parse FIT file")),
    };

    // iterate over all data elements
    for data in parsed_data {
        match data.kind() {
            // extract the timestamp of the activity and check it is an activity
            fitparser::profile::field_types::MesgNum::FileId => {
                for field in data.fields() {
                    match field.name() {
                        "time_created" => match &field.value() {
                            fitparser::Value::Timestamp(val) => activity_data.timestamp = *val,
                            &_ => {
                                return Err(String::from(
                                    "unexpected value in enum fitparser::Value",
                                ))
                            }
                        },
                        "type" => match &field.value() {
                            fitparser::Value::String(_val) => continue,
                            &_ => {
                                return Err(String::from(
                                    "unexpected value in enum fitparser::Value",
                                ))
                            }
                        },
                        &_ => (), // ignore all other values
                    }
                }
            }

            // extract the sport type of the activity
            fitparser::profile::field_types::MesgNum::Sport => {
                for field in data.fields() {
                    match field.name() {
                        "sport" => match &field.value() {
                            fitparser::Value::String(val) => {
                                activity_data.sport = val.to_string();
                            }
                            &_ => {
                                return Err(String::from(
                                    "unexpected value in enum fitparser::Value",
                                ))
                            }
                        },
                        &_ => (), // ignore all other values
                    }
                }
            }
            _ => (), // ignore all other values
        }
    }
    Ok(activity_data)
}

fn main() -> ExitCode {
    println!(
        "Parsing FIT files using Profile version: {}",
        fitparser::profile::VERSION
    );

    let filename = String::from("/tmp/activity.fit");

    match parse_fit_file(filename) {
        Ok(val) => {
            println!("file: {}", val.timestamp.format("%Y%m%dT%H%M%SZ"));
            println!("directory: {}", val.timestamp.format("%Y/%m"));
            println!("sport: {}", val.sport);
        }
        Err(msg) => println!("ERROR: {}", msg),
    };

    ExitCode::SUCCESS
}
