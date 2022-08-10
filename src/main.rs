use aho_corasick::AhoCorasick;
use chrono::{DateTime, TimeZone, Utc};
use clap::{Arg, Command};
use std::fs::{self, File};
use std::process::ExitCode;

struct ActivityData {
    sport: String,
    sub_sport: String,
    timestamp: DateTime<Utc>,
}

impl ActivityData {
    fn new() -> ActivityData {
        ActivityData {
            sport: String::from("unknown"),
            sub_sport: String::from("unknown"),
            timestamp: chrono::Utc.ymd(1970, 1, 1).and_hms(0, 0, 0),
        }
    }
}

fn expand_formatstring(formatstring: &str, activity_data: &ActivityData) -> String {
    // first define the mappings as slice for better visibility ...
    let mappings = &[
        &["$s", activity_data.sport.as_str()],
        &["$S", activity_data.sub_sport.as_str()],
    ];

    // then convert the slice to the required vectors
    let mut placeholders: Vec<&str> = vec![];
    let mut substitutions: Vec<&str> = vec![];
    for mapping in mappings {
        placeholders.push(mapping[0]);
        substitutions.push(mapping[1]);
    }

    // replace all '$' placeholders with their substitutions (activity)
    let result = AhoCorasick::new(placeholders).replace_all(formatstring, &substitutions);

    // replace all '%' placeholders with their substitions (timestamp)
    activity_data
        .timestamp
        .format(&result.to_string())
        .to_string()
}

fn parse_fit_file(filename: &str) -> Result<ActivityData, String> {
    let mut activity_data = ActivityData::new();

    // open FIT file
    let mut fp = match File::open(filename) {
        Ok(fp) => fp,
        Err(_err) => return Err(format!("Unable to open '{}'", filename)),
    };

    // parse FIT file to data structure
    let parsed_data = match fitparser::from_reader(&mut fp) {
        Ok(parsed_data) => parsed_data,
        Err(_err) => return Err(format!("Unable to parse '{}'", filename)),
    };

    // iterate over all data elements
    for data in parsed_data {
        match data.kind() {
            // extract the timestamp of the activity and check it is an activity
            fitparser::profile::field_types::MesgNum::FileId => {
                for field in data.fields() {
                    match field.name() {
                        "time_created" => match &field.value() {
                            fitparser::Value::Timestamp(val) => {
                                activity_data.timestamp = DateTime::from(*val)
                            }
                            &_ => {
                                return Err(format!(
                                    "Unexpected value in enum fitparser::Value in '{}'",
                                    filename
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
                                return Err(format!(
                                    "Unexpected value in enum fitparser::Value in '{}'",
                                    filename
                                ))
                            }
                        },
                        "sub_sport" => match &field.value() {
                            fitparser::Value::String(val) => {
                                activity_data.sub_sport = val.to_string();
                            }
                            &_ => {
                                return Err(format!(
                                    "Unexpected value in enum fitparser::Value in '{}'",
                                    filename
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

fn parse_arguments() -> clap::ArgMatches {
    Command::new("FIT file archiver")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("directory")
                .short('d')
                .long("directory")
                .takes_value(true)
                .value_name("base directory")
                .default_value(".")
                .help("Base directory where the archive is created"),
        )
        .arg(
            Arg::with_name("filename_format")
                .short('F')
                .long("file-format")
                .takes_value(true)
                .value_name("format string")
                .default_value("%Y-%m-%d-%H%M%S-$s")
                .help("Format string defining the name of the archive file"),
        )
        .arg(
            Arg::with_name("directory_format")
                .short('D')
                .long("directory-format")
                .takes_value(true)
                .value_name("format string")
                .default_value("%Y/%m")
                .help("Format string defining the path of the archive directory"),
        )
        .arg(
            Arg::with_name("move")
                .short('m')
                .long("move")
                .takes_value(false)
                .help("Move instead of copying files to archive"),
        )
        .arg(
            Arg::with_name("files")
                .multiple(true)
                .value_name("files")
                .required(true)
                .help("FIT files to archive"),
        )
        .get_matches()
}

fn process_files(options: clap::ArgMatches) -> Result<String, String> {
    let mut file_counter: u16 = 0;
    let mut error_counter: u16 = 0;

    let destination = options.value_of("directory").unwrap();
    let filenames: Vec<&str> = options.values_of("files").unwrap().collect();

    for filename in filenames {
        match parse_fit_file(filename) {
            Ok(val) => {
                let archive_path = format!(
                    "{}/{}",
                    destination,
                    expand_formatstring(options.value_of("directory_format").unwrap(), &val)
                );
                let archive_file = format!(
                    "{}/{}.fit",
                    archive_path,
                    expand_formatstring(options.value_of("filename_format").unwrap(), &val)
                );

                // Check if destination exists and is a directory, create it if needed
                match fs::metadata(&archive_path) {
                    Ok(val) => {
                        if !val.is_dir() {
                            return Err(format!(
                                "'{}' exists but is not a directory",
                                archive_path
                            ));
                        }
                    }
                    Err(_) => {
                        print!("Creating directory '{}' ... ", archive_path);
                        match fs::create_dir_all(&archive_path) {
                            Ok(_) => println!("done"),
                            Err(_) => {
                                return Err(format!(
                                    "Unable to create archive directory '{}'",
                                    archive_path
                                ))
                            }
                        }
                    }
                }

                // Archiving files
                if options.is_present("move") {
                    print!("Moving '{}' -> '{}' ... ", filename, archive_file);
                    match fs::copy(&filename, &archive_file) {
                        Ok(_) => match fs::remove_file(&filename) {
                            Ok(_) => {
                                println!("done");
                                file_counter += 1;
                            }
                            Err(_) => {
                                println!("Unable to remove file '{}'", filename);
                                error_counter += 1;
                            }
                        },
                        Err(_) => {
                            println!("Unable to create file '{}'", archive_file);
                            error_counter += 1;
                        }
                    }
                } else {
                    print!("Copying '{}' -> '{}' ... ", filename, archive_file);
                    match fs::copy(&filename, &archive_file) {
                        Ok(_) => {
                            println!("done");
                            file_counter += 1;
                        }
                        Err(_) => {
                            println!("Unable to create file '{}'", archive_file);
                            error_counter += 1;
                        }
                    }
                }
            }
            Err(msg) => println!("{}", msg),
        };
    }

    Ok(format!(
        "Processed {} files with {} errors",
        file_counter, error_counter
    ))
}

fn main() -> ExitCode {
    match process_files(parse_arguments()) {
        Ok(val) => println!("{}", val),
        Err(val) => println!("{}", val),
    };

    ExitCode::SUCCESS
}
