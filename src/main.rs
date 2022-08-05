use chrono::{DateTime, Local, TimeZone};
use clap::{Arg, Command};
use std::fs::File;
use std::process::ExitCode;

struct ActivityData {
    sport: String,
    timestamp: DateTime<Local>,
}

fn parse_fit_file(filename: &str) -> Result<ActivityData, String> {
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
    activity_data.timestamp = match parsed_data
        .iter()
        .find(|data| data.kind() == fitparser::profile::field_types::MesgNum::FileId)
        .unwrap()
        .fields()
        .iter()
        .find(|field| field.name() == "time_created")
        .unwrap()
        .value()
    {
        fitparser::Value::Timestamp(val) => *val,
        &_ => return Err(String::from("unexpected value in enum fitparser::Value")),
    };
    activity_data.sport = match parsed_data
        .iter()
        .find(|data| data.kind() == fitparser::profile::field_types::MesgNum::Sport)
        .unwrap()
        .fields()
        .iter()
        .find(|field| field.name() == "sport")
        .unwrap()
        .value()
    {
        fitparser::Value::String(val) => val.to_string(),
        &_ => return Err(String::from("unexpected value in enum fitparser::Value")),
    };

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

fn main() -> ExitCode {
    let options = parse_arguments();

    let filenames: Vec<&str> = options.values_of("files").unwrap().collect();
    println!("filenames: {}", filenames.join(" "));
    let destination = options.value_of("directory").unwrap();
    println!("directory: {}", destination);
    println!("move: {:#?}", options.is_present("move"));

    println!(
        "Parsing FIT files using Profile version: {}",
        fitparser::profile::VERSION
    );

    for filename in filenames {
        match parse_fit_file(filename) {
            Ok(val) => {
                println!(
                    "{} -> {}/{}/{}-{}.fit",
                    filename,
                    destination,
                    val.timestamp.format("%Y/%m"),
                    val.timestamp.format("%Y-%m-%d-%H%M%S"),
                    val.sport
                );
            }
            Err(msg) => println!("ERROR: {}", msg),
        };
    }


    ExitCode::SUCCESS
}
