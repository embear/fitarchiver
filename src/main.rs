use aho_corasick::AhoCorasick;
use chrono::{DateTime, TimeZone, Utc};
use clap::{Arg, Command};
use std::fs::{self, File};
use std::path::Path;
use std::process::ExitCode;

struct ActivityData {
    sport: String,
    sport_name: String,
    sub_sport: String,
    workout_name: String,
    timestamp: DateTime<Utc>,
}

impl ActivityData {
    fn new() -> ActivityData {
        ActivityData {
            sport: String::from("unknown"),
            sport_name: String::from("unknown"),
            sub_sport: String::from("unknown"),
            workout_name: String::from("unknown"),
            timestamp: chrono::Utc.ymd(1970, 1, 1).and_hms(0, 0, 0),
        }
    }
}

fn expand_formatstring(formatstring: &str, activity_data: &ActivityData) -> String {
    // the following code is not the most efficient one but makes the mappings obvious

    // first define the mappings as slice for better visibility ...
    let mappings = &[
        &["$s", activity_data.sport.as_str()],
        &["$n", activity_data.sport_name.as_str()],
        &["$S", activity_data.sub_sport.as_str()],
        &["$w", activity_data.workout_name.as_str()],
    ];

    // ... then convert the slice to the required vectors
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

fn parse_fit_file(path: &Path) -> Result<ActivityData, String> {
    let mut activity_data = ActivityData::new();

    // open FIT file
    let mut fp = match File::open(path) {
        Ok(fp) => fp,
        Err(_err) => return Err(format!("Unable to open '{}'", path.display())),
    };

    // parse FIT file to data structure
    let parsed_data = match fitparser::from_reader(&mut fp) {
        Ok(parsed_data) => parsed_data,
        Err(_err) => return Err(format!("Unable to parse '{}'", path.display())),
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
                                    path.display()
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
                        "name" => match &field.value() {
                            fitparser::Value::String(val) => {
                                activity_data.sport_name =
                                    val.to_lowercase().replace(' ', "_").to_string();
                            }
                            &_ => {
                                return Err(format!(
                                    "Unexpected value in enum fitparser::Value in '{}'",
                                    path.display()
                                ))
                            }
                        },
                        "sport" => match &field.value() {
                            fitparser::Value::String(val) => {
                                activity_data.sport =
                                    val.to_lowercase().replace(' ', "_").to_string();
                            }
                            &_ => {
                                return Err(format!(
                                    "Unexpected value in enum fitparser::Value in '{}'",
                                    path.display()
                                ))
                            }
                        },
                        "sub_sport" => match &field.value() {
                            fitparser::Value::String(val) => {
                                activity_data.sub_sport =
                                    val.to_lowercase().replace(' ', "_").to_string();
                            }
                            &_ => {
                                return Err(format!(
                                    "Unexpected value in enum fitparser::Value in '{}'",
                                    path.display()
                                ))
                            }
                        },
                        &_ => (), // ignore all other values
                    }
                }
            }

            // extract the wkt_name of the activity
            fitparser::profile::field_types::MesgNum::Workout => {
                for field in data.fields() {
                    match field.name() {
                        "wkt_name" => match &field.value() {
                            fitparser::Value::String(val) => {
                                activity_data.workout_name =
                                    val.to_lowercase().replace(' ', "_").to_string();
                            }
                            &_ => {
                                return Err(format!(
                                    "Unexpected value in enum fitparser::Value in '{}'",
                                    path.display()
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
            Arg::with_name("file_template")
                .short('f')
                .long("file-template")
                .takes_value(true)
                .value_name("template string")
                .default_value("%Y/%m/%Y-%m-%d-%H%M%S-$s")
                .help("Format string defining the path and name of the archive file in the destination directory.")
                .long_help(
"Format string defining the path and name of the archive file in the destination
directory.
For expanding the timestamp of the workout all conversions of strftime() are
supported. In addition to those the converstion the following FIT file specific
conversions are supported:
  $s  sport type, 'unknown' if not available.
  $n  sport name, 'unknown' if not available.
  $S  sport subtype, 'unknown' if not available.
  $w  workout name, 'unknown' if not available.")
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

    let base_directory = Path::new(options.value_of("directory").unwrap());
    let files: Vec<&str> = options.values_of("files").unwrap().collect();

    for file in files {
        let source_path = Path::new(file);
        match parse_fit_file(source_path) {
            Ok(val) => {
                let archive_path = base_directory
                    .join(expand_formatstring(
                        options.value_of("file_template").unwrap(),
                        &val,
                    ))
                    .with_extension("fit");

                // check if destination exists and is a directory, create it if needed
                let parent = archive_path.parent().unwrap();
                match fs::metadata(parent) {
                    Ok(val) => {
                        if !val.is_dir() {
                            return Err(format!(
                                "'{}' exists but is not a directory",
                                parent.display()
                            ));
                        }
                    }
                    Err(_) => {
                        print!("Creating directory '{}' ... ", parent.display());
                        match fs::create_dir_all(&parent) {
                            Ok(_) => println!("done"),
                            Err(_) => {
                                return Err(format!(
                                    "Unable to create archive directory '{}'",
                                    parent.display()
                                ))
                            }
                        }
                    }
                }

                // archiving files
                print!(
                    "'{}' -> '{}' ... ",
                    source_path.display(),
                    archive_path.display()
                );
                match fs::copy(&source_path, &archive_path) {
                    Ok(_) => {
                        if options.is_present("move") {
                            match fs::remove_file(&source_path) {
                                Ok(_) => {
                                    println!("moved");
                                    file_counter += 1;
                                }
                                Err(_) => {
                                    eprintln!("Unable to remove file '{}'", source_path.display());
                                    error_counter += 1;
                                }
                            }
                        } else {
                            println!("copied");
                            file_counter += 1;
                        }
                    }
                    Err(_) => {
                        eprintln!("Unable to create file '{}'", archive_path.display());
                        error_counter += 1;
                    }
                }
            }
            Err(msg) => eprintln!("{}", msg),
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
        Err(val) => eprintln!("{}", val),
    };

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_formatstring() {
        let mut activity_data = ActivityData::new();

        // empty format string
        assert_eq!(String::from(""), expand_formatstring("", &activity_data));

        // default format string
        assert_eq!(
            String::from("1970/01/1970-01-01-000000-unknown"),
            expand_formatstring("%Y/%m/%Y-%m-%d-%H%M%S-$s", &activity_data)
        );

        // single activity data
        assert_eq!(
            String::from("unknown"),
            expand_formatstring("$s", &activity_data)
        );
        assert_eq!(
            String::from("unknown"),
            expand_formatstring("$n", &activity_data)
        );
        assert_eq!(
            String::from("unknown"),
            expand_formatstring("$S", &activity_data)
        );
        assert_eq!(
            String::from("unknown"),
            expand_formatstring("$w", &activity_data)
        );

        // change activity data
        activity_data.sport = String::from("running");
        activity_data.sport_name = String::from("training");
        activity_data.sub_sport = String::from("trail");
        activity_data.workout_name = String::from("interval");
        activity_data.timestamp = Utc.ymd(2014, 7, 8).and_hms(9, 10, 11);

        // default format string
        assert_eq!(
            String::from("2014/07/2014-07-08-091011-running"),
            expand_formatstring("%Y/%m/%Y-%m-%d-%H%M%S-$s", &activity_data)
        );

        assert_eq!(
            String::from("running"),
            expand_formatstring("$s", &activity_data)
        );
        assert_eq!(
            String::from("training"),
            expand_formatstring("$n", &activity_data)
        );
        assert_eq!(
            String::from("trail"),
            expand_formatstring("$S", &activity_data)
        );
        assert_eq!(
            String::from("interval"),
            expand_formatstring("$w", &activity_data)
        );

        // repeated templates
        assert_eq!(
            String::from("running-running-running-running"),
            expand_formatstring("$s-$s-$s-$s", &activity_data)
        );
    }
}
