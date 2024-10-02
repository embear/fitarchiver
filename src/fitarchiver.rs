//! # FIT file archiver module

#![warn(missing_docs)]

use aho_corasick::AhoCorasick;
use chrono::{DateTime, TimeZone, Utc};
use clap::{Arg, ArgAction, Command};
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::path::Path;

#[derive(Debug)]
pub struct ArchiverError {
    details: String,
}

impl ArchiverError {
    fn new(msg: &str) -> ArchiverError {
        ArchiverError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for ArchiverError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for ArchiverError {
    fn description(&self) -> &str {
        &self.details
    }
}

type Result<T> = std::result::Result<T, ArchiverError>;

/// Information extracted from a FIT file
#[derive(Debug)]
struct ActivityData {
    /// Sport type, i.e. 'running'
    sport: String,
    /// Sport name, i.e. 'trail_run' (Name of the activity started on the watch)
    sport_name: String,
    /// Sport sub type, i.e. 'trail'
    sub_sport: String,
    /// Workout name, i.e. 'temporun_8km'
    workout_name: String,
    /// UTC timestamp of activity start
    timestamp: DateTime<Utc>,
}

impl ActivityData {
    /// Returns an initialized activity data structure with default values
    fn new() -> ActivityData {
        ActivityData {
            sport: String::from("unknown"),
            sport_name: String::from("unknown"),
            sub_sport: String::from("unknown"),
            workout_name: String::from("unknown"),
            timestamp: chrono::Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap(),
        }
    }
}

/// Returns an expanded format string with '%' and '$' replaced
///
/// '%' tag are expanded using the timestamp of the acticity data. The '$' tag
/// are expanded using other data from the activity.
///
/// # Arguments
///
/// * `formatstring` - A format string containing '%' and '$' tags.
/// * `activity_data` - Data that will be used for expansion of the tags.
fn expand_formatstring(formatstring: &str, activity_data: &ActivityData) -> String {
    // the following code is not the most efficient one but makes the mappings obvious

    // first define the mappings as slice for better visibility ...
    let mappings = [
        ["$s", activity_data.sport.as_str()],
        ["$n", activity_data.sport_name.as_str()],
        ["$S", activity_data.sub_sport.as_str()],
        ["$w", activity_data.workout_name.as_str()],
    ];

    // ... then convert the slice to the required vectors
    let tags: Vec<&str> = mappings.iter().map(|x| x[0]).collect();
    let substitutions: Vec<&str> = mappings.iter().map(|x| x[1]).collect();

    // replace all '$' tags with their substitutions (activity)
    let result = AhoCorasick::new(tags)
        .unwrap()
        .replace_all(formatstring, &substitutions);

    // replace all '%' tags with their substitions (timestamp)
    activity_data
        .timestamp
        .format(&result.to_string())
        .to_string()
}

/// Returns activity data extracted from given FIT file
///
/// # Arguments
///
/// * `path` - Path of the FIT file
fn parse_fit_file(path: &Path) -> Result<ActivityData> {
    let mut activity_data = ActivityData::new();
    let mut sports: Vec<String> = Vec::new();

    // open FIT file
    let mut fp = match File::open(path) {
        Ok(fp) => fp,
        Err(_err) => {
            let msg = format!("Unable to open '{}'", path.display());
            return Err(ArchiverError::new(&msg));
        }
    };

    // parse FIT file to data structure
    let parsed_data = match fitparser::from_reader(&mut fp) {
        Ok(parsed_data) => parsed_data,
        Err(_err) => {
            let msg = format!("Unable to parse '{}'", path.display());
            return Err(ArchiverError::new(&msg));
        }
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
                                let msg = format!(
                                    "Unexpected value '{}' in enum fitparser::Value '{}' in '{}'",
                                    field.value(),
                                    field.name(),
                                    path.display()
                                );
                                return Err(ArchiverError::new(&msg));
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
                                    val.trim().to_lowercase().replace(' ', "_").to_string();
                            }
                            &_ => {
                                eprintln!(
                                    "Unexpected value '{}' in enum fitparser::Value '{}' in '{}'. Using 'unknown' instead!",
                                    field.value(),
                                    field.name(),
                                    path.display()
                                );
                            }
                        },
                        "sport" => match &field.value() {
                            fitparser::Value::String(val) => {
                                sports
                                    .push(val.trim().to_lowercase().replace(' ', "_").to_string());
                            }
                            &_ => {
                                eprintln!(
                                    "Unexpected value '{}' in enum fitparser::Value '{}' in '{}'. Using 'unknown' instead!",
                                    field.value(),
                                    field.name(),
                                    path.display()
                                );
                            }
                        },
                        "sub_sport" => match &field.value() {
                            fitparser::Value::String(val) => {
                                activity_data.sub_sport =
                                    val.trim().to_lowercase().replace(' ', "_").to_string();
                            }
                            &_ => {
                                eprintln!(
                                    "Unexpected value '{}' in enum fitparser::Value '{}' in '{}'. Using 'unknown' instead!",
                                    field.value(),
                                    field.name(),
                                    path.display()
                                );
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
                                    val.trim().to_lowercase().replace(' ', "_").to_string();
                            }
                            &_ => {
                                eprintln!(
                                    "Unexpected value '{}' in enum fitparser::Value '{}' in '{}'. Using 'unknown' instead!",
                                    field.value(),
                                    field.name(),
                                    path.display()
                                );
                            }
                        },
                        &_ => (), // ignore all other values
                    }
                }
            }

            _ => (), // ignore all other values
        }
    }

    // build sport value for single- and multisport activities
    if sports.len() == 1 {
        activity_data.sport = sports.get(0).unwrap().to_string();
    } else if sports.len() > 1 {
        activity_data.sport = String::from("multisport_") + &sports.join("_");
    }

    Ok(activity_data)
}

/// Returns matched command line arguments
pub fn parse_arguments(arguments: Option<Vec<&str>>) -> clap::ArgMatches {
    const VERSION: &'static str = concat!(
        env!("VERGEN_SEMVER"),
        " compiled at ",
        env!("VERGEN_BUILD_TIMESTAMP")
    );
    let parser = Command::new("FIT file archiver")
        .version(VERSION)
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .num_args(1)
                .value_name("archive directory")
                .default_value(".")
                .help("Archive base directory.")
                .long_help("Base directory where the archive is created."),
        )
        .arg(
            Arg::new("file-template")
                .short('f')
                .long("file-template")
                .num_args(1)
                .value_name("template string")
                .default_value("%Y/%m/%Y-%m-%d-%H%M%S-$s")
                .help("Format string defining the path and name of the archive file in the archive directory.")
                .long_help(
"Format template that defines the path and name of the archive file in the archive directory. '/' must be used as a separator for path components. All strftime() tags are supported for expanding the time information of the training. In addition to the time information the following FIT file specific expansions are supported:

  Tag   Description     Example          Default
  ------------------------------------------------
  $s    sport type      'running'        'unknown'
  $S    sport subtype   'trail'          'unknown'
  $n    sport name      'trail_run'      'unknown'
  $w    workout name    'temporun_8km'   'unknown'

NOTE: It is possible that the shell used tries to replace tags. Therefore, the template should be passed as a quoted string.")
        )
        .arg(
            Arg::new("move")
                .short('m')
                .long("move")
                .action(ArgAction::SetTrue)
                .help("Move files to archive instead of copying them."),
        )
        .arg(
            Arg::new("dry-run")
                .short('n')
                .long("dry-run")
                .action(ArgAction::SetTrue)
                .help("Do not copy or move the files, just show what will happen."),
        )
        .arg(
            Arg::new("files")
                .num_args(1..)
                .value_name("files")
                .required(true)
                .help("List of FIT files to archive."),
        );

    match arguments {
        Some(val) => parser.get_matches_from(val),
        None => parser.get_matches(),
    }
}

/// Create directory for archive file.
///
/// # Arguments
///
/// `archive_path` - Path to the archive file.
/// `options` - Command line options.
fn create_archive_directory(archive_path: &Path, options: &clap::ArgMatches) -> Result<String> {
    // check if destination exists and is a directory, create it if needed
    match archive_path.parent() {
        Some(parent) => match fs::metadata(parent) {
            Ok(val) => {
                if !val.is_dir() {
                    let msg = format!("'{}' exists but is not a directory", parent.display());
                    return Err(ArchiverError::new(&msg));
                }
            }
            Err(_) => {
                if !options.get_flag("dry-run") {
                    match fs::create_dir_all(&parent) {
                        Ok(_) => (),
                        Err(_) => {
                            let msg = format!(
                                "Unable to create archive directory '{}'",
                                parent.display()
                            );
                            return Err(ArchiverError::new(&msg));
                        }
                    }
                }
            }
        },
        None => {
            let msg = format!(
                "'{}' is not contained in a directory",
                archive_path.display()
            );
            return Err(ArchiverError::new(&msg));
        }
    }
    Ok(String::from("OK"))
}

/// Move or copy files
///
/// # Arguments
///
/// `source_path` - Path to the source file.
/// `archive_path` - Path to the archive file.
/// `options` - Command line options.
fn archive_file(
    source_path: &Path,
    archive_path: &Path,
    options: &clap::ArgMatches,
) -> Result<String> {
    let mut msg = format!(
        "'{}' -> '{}' ... ",
        source_path.display(),
        archive_path.display()
    );
    if !options.get_flag("dry-run") {
        match fs::copy(source_path, &archive_path) {
            Ok(_) => {
                if options.get_flag("move") {
                    match fs::remove_file(source_path) {
                        Ok(_) => {
                            msg.push_str("moved");
                        }
                        Err(_) => {
                            let msg = format!("Unable to remove file '{}'", source_path.display());
                            return Err(ArchiverError::new(&msg));
                        }
                    }
                } else {
                    msg.push_str("copied");
                }
            }
            Err(_) => {
                let msg = format!("Unable to create file '{}'", archive_path.display());
                return Err(ArchiverError::new(&msg));
            }
        };
    } else {
        msg.push_str("dry run");
    }
    Ok(msg)
}

/// Process all FIT files
///
/// # Arguments
///
/// `options` - Command line options.
pub fn process_files(options: &clap::ArgMatches) -> Result<String> {
    let mut file_counter: u16 = 0;
    let mut error_counter: u16 = 0;

    let base_directory = Path::new(options.get_one::<String>("directory").unwrap().as_str());
    let files: Vec<&str> = options
        .get_many::<String>("files")
        .unwrap()
        .map(|s| s.as_str())
        .collect();

    for file in files {
        let source_path = Path::new(file);
        match parse_fit_file(source_path) {
            Ok(val) => {
                let archive_path = base_directory
                    .join(expand_formatstring(
                        options.get_one::<String>("file-template").unwrap().as_str(),
                        &val,
                    ))
                    .with_extension("fit");

                match create_archive_directory(&archive_path, options) {
                    Ok(_) => match archive_file(source_path, &archive_path, options) {
                        Ok(msg) => {
                            println!("{}", msg);
                            file_counter += 1;
                        }
                        Err(msg) => {
                            eprintln!("{}", msg);
                            error_counter += 1;
                        }
                    },
                    Err(e) => return Err(e),
                }
            }
            Err(msg) => eprintln!("{}", msg),
        };
    }

    let msg = format!("Processed {} files", file_counter);
    let err = if error_counter == 0 {
        String::new()
    } else {
        format!("with {} errors.", error_counter)
    };

    Ok([msg, err].join(" "))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use std::fs::{self, File};
    use std::path::PathBuf;
    use tempdir::TempDir;

    #[test]
    /// Test format string expansion
    fn test_expand_formatstring() {
        // setup activity data
        let activity_data = super::ActivityData {
            sport: String::from("running"),
            sport_name: String::from("training"),
            sub_sport: String::from("trail"),
            workout_name: String::from("interval"),
            timestamp: chrono::Utc.with_ymd_and_hms(2014, 7, 8, 9, 10, 11).unwrap(),
        };

        // default format string
        assert_eq!(
            String::from("2014/07/2014-07-08-091011-running"),
            super::expand_formatstring("%Y/%m/%Y-%m-%d-%H%M%S-$s", &activity_data)
        );

        // single tags
        assert_eq!(
            String::from("running"),
            super::expand_formatstring("$s", &activity_data)
        );
        assert_eq!(
            String::from("training"),
            super::expand_formatstring("$n", &activity_data)
        );
        assert_eq!(
            String::from("trail"),
            super::expand_formatstring("$S", &activity_data)
        );
        assert_eq!(
            String::from("interval"),
            super::expand_formatstring("$w", &activity_data)
        );

        // repeated tags
        assert_eq!(
            String::from("running-running-running-running"),
            super::expand_formatstring("$s-$s-$s-$s", &activity_data)
        );
    }

    #[test]
    // Test creating the archive directory
    fn test_create_archive_directory() {
        let tmpdir = TempDir::new("fitarchive").expect("Error during creating temporary directory");
        let source_path = tmpdir.path().join("source_dir").join("source.fit");
        let archive_file = tmpdir.path().join("archive_dir").join("archive.fit");

        let options = super::parse_arguments(Some(vec![
            "fitarchiver",
            "-d",
            archive_file.parent().unwrap().as_os_str().to_str().unwrap(),
            source_path.as_os_str().to_str().unwrap(),
        ]));

        assert!(!archive_file.parent().unwrap().exists());
        super::create_archive_directory(&archive_file, &options)
            .expect("error during creating directory");
        assert!(archive_file.parent().unwrap().exists());

        // cleanup
        fs::remove_dir_all(&tmpdir).expect("error during removing temporary directory");
    }

    #[test]
    // Test not creating the archive directory in dry run
    fn test_create_archive_directory_dry_run() {
        let tmpdir = TempDir::new("fitarchive").expect("Error during creating temporary directory");
        let source_path = tmpdir.path().join("source_dir").join("source.fit");
        let archive_file = tmpdir.path().join("archive_dir").join("archive.fit");

        let options = super::parse_arguments(Some(vec![
            "fitarchiver",
            "-n",
            "-d",
            archive_file.parent().unwrap().as_os_str().to_str().unwrap(),
            source_path.as_os_str().to_str().unwrap(),
        ]));

        assert!(!archive_file.parent().unwrap().exists());
        super::create_archive_directory(&archive_file, &options)
            .expect("error during creating directory");
        assert!(!archive_file.parent().unwrap().exists());

        // cleanup
        fs::remove_dir_all(&tmpdir).expect("error during removing temporary directory");
    }

    #[test]
    // Test failure when parent of archive directory is missing
    fn test_create_archive_directory_failure_parent_missing() {
        let tmpdir = TempDir::new("fitarchive").expect("Error during creating temporary directory");
        let source_path = tmpdir.path().join("source_dir").join("source.fit");
        let archive_path = PathBuf::new();

        let options = super::parse_arguments(Some(vec![
            "fitarchiver",
            "-d",
            archive_path.as_os_str().to_str().unwrap(),
            source_path.as_os_str().to_str().unwrap(),
        ]));

        super::create_archive_directory(&archive_path, &options).expect_err("error expected");

        // cleanup
        fs::remove_dir_all(&tmpdir).expect("error during removing temporary directory");
    }

    #[test]
    // Test failure when archive directory is a file
    fn test_create_archive_directory_failure_file_exists() {
        let tmpdir = TempDir::new("fitarchive").expect("Error during creating temporary directory");
        let source_path = tmpdir.path().join("source_dir").join("source.fit");
        let archive_file = tmpdir.path().join("archive_dir").join("archive.fit");

        let options = super::parse_arguments(Some(vec![
            "fitarchiver",
            "-d",
            archive_file.parent().unwrap().as_os_str().to_str().unwrap(),
            source_path.as_os_str().to_str().unwrap(),
        ]));

        std::fs::File::create(&archive_file.parent().unwrap())
            .expect("error during creating directory");
        super::create_archive_directory(&archive_file, &options).expect_err("error expected");

        // cleanup
        fs::remove_dir_all(&tmpdir).expect("error during removing temporary directory");
    }

    #[test]
    // Test failure when archive directory is not a directory
    fn test_create_archive_directory_failure_unable_to_create() {
        let tmpdir = TempDir::new("fitarchive").expect("Error during creating temporary directory");
        let source_path = tmpdir.path().join("source_dir").join("source.fit");
        let archive_file = PathBuf::from("/").join("archive_dir").join("archive.fit");

        let options = super::parse_arguments(Some(vec![
            "fitarchiver",
            "-d",
            archive_file.parent().unwrap().as_os_str().to_str().unwrap(),
            source_path.as_os_str().to_str().unwrap(),
        ]));

        super::create_archive_directory(&archive_file, &options).expect_err("error expected");

        // cleanup
        fs::remove_dir_all(&tmpdir).expect("error during removing temporary directory");
    }

    #[test]
    /// Test dry run
    fn test_archive_file_dry_run() {
        let tmpdir = TempDir::new("fitarchive").expect("Error during creating temporary directory");
        let source_path = tmpdir.path().join("source_dir").join("source.fit");
        let archive_file = tmpdir.path().join("archive_dir").join("archive.fit");

        {
            // put file creation into a separate scope so the file is closed for the actual test
            fs::create_dir_all(&source_path.parent().unwrap())
                .expect("error during creating temporary archive directory");
            fs::create_dir_all(&archive_file.parent().unwrap())
                .expect("error during creating temporary archive directory");
            File::create(&source_path).expect("unable to create test file");
        }

        let options = super::parse_arguments(Some(vec![
            "fitarchiver",
            "-n",
            "-d",
            archive_file.parent().unwrap().as_os_str().to_str().unwrap(),
            "-f",
            "archive",
            source_path.as_os_str().to_str().unwrap(),
        ]));

        assert!(source_path.exists());
        assert!(!archive_file.exists());
        super::archive_file(&source_path, &archive_file, &options)
            .expect("error during archiving file");
        assert!(source_path.exists());
        assert!(!archive_file.exists());

        // cleanup
        fs::remove_dir_all(&tmpdir).expect("error during removing temporary directory");
    }

    #[test]
    /// Test copying file to archive
    fn test_archive_file_copy() {
        let tmpdir = TempDir::new("fitarchive").expect("Error during creating temporary directory");
        let source_path = tmpdir.path().join("source_dir").join("source.fit");
        let archive_file = tmpdir.path().join("archive_dir").join("archive.fit");

        {
            // put file creation into a separate scope so the file is closed for the actual test
            fs::create_dir_all(&source_path.parent().unwrap())
                .expect("error during creating temporary archive directory");
            fs::create_dir_all(&archive_file.parent().unwrap())
                .expect("error during creating temporary archive directory");
            File::create(&source_path).expect("unable to create test file");
        }

        let options = super::parse_arguments(Some(vec![
            "fitarchiver",
            "-d",
            archive_file.parent().unwrap().as_os_str().to_str().unwrap(),
            "-f",
            "archive",
            source_path.as_os_str().to_str().unwrap(),
        ]));

        assert!(source_path.exists());
        assert!(!archive_file.exists());
        super::archive_file(&source_path, &archive_file, &options)
            .expect("error during archiving file");
        assert!(source_path.exists());
        assert!(archive_file.exists());

        // cleanup
        fs::remove_dir_all(&tmpdir).expect("error during removing temporary directory");
    }

    #[test]
    /// Test moving file to archive
    fn test_archive_file_move() {
        let tmpdir = TempDir::new("fitarchive").expect("Error during creating temporary directory");
        let source_path = tmpdir.path().join("source_dir").join("source.fit");
        let archive_file = tmpdir.path().join("archive_dir").join("archive.fit");

        {
            // put file creation into a separate scope so the file is closed for the actual test
            fs::create_dir_all(&source_path.parent().unwrap())
                .expect("error during creating temporary archive directory");
            fs::create_dir_all(&archive_file.parent().unwrap())
                .expect("error during creating temporary archive directory");
            File::create(&source_path).expect("unable to create test file");
        }

        let options = super::parse_arguments(Some(vec![
            "fitarchiver",
            "-m",
            "-d",
            archive_file.parent().unwrap().as_os_str().to_str().unwrap(),
            "-f",
            "archive",
            source_path.as_os_str().to_str().unwrap(),
        ]));

        assert!(source_path.exists());
        assert!(!archive_file.exists());
        super::archive_file(&source_path, &archive_file, &options)
            .expect("error during archiving file");
        assert!(!source_path.exists());
        assert!(archive_file.exists());

        // cleanup
        fs::remove_dir_all(&tmpdir).expect("error during removing temporary directory");
    }

    #[test]
    /// Test extracting activity data from real FIT file
    fn test_activity_data_from_file() {
        // get the directory of the test executable
        let mut source_path = std::env::current_exe()
            .unwrap()
            .parent()
            .expect("executable directory")
            .to_path_buf();

        // go up to the repository base directory
        source_path.pop();
        source_path.pop();
        source_path.pop();

        // append location of the test data
        source_path.push("test");
        source_path.push("test_data_01.fit");

        let result = super::parse_fit_file(&source_path);
        assert!(result.is_ok());
        let activity_data = result.unwrap();
        assert_eq!(String::from("running"), activity_data.sport);
        assert_eq!(String::from("trail_run"), activity_data.sport_name);
        assert_eq!(String::from("trail"), activity_data.sub_sport);
        assert_eq!(String::from("test_workout"), activity_data.workout_name);
        assert_eq!(
            chrono::Utc.with_ymd_and_hms(2023, 7, 26, 6, 22, 4).unwrap(),
            activity_data.timestamp
        );
    }

    #[test]
    /// Test activity file is missing
    fn test_activity_data_from_file_failure_file_missing() {
        // get the directory of the test executable
        let mut source_path = std::env::current_exe()
            .unwrap()
            .parent()
            .expect("executable directory")
            .to_path_buf();

        // go up to the repository base directory
        source_path.pop();
        source_path.pop();
        source_path.pop();

        // append location of the test data
        source_path.push("test");
        source_path.push("missing.fit");

        super::parse_fit_file(&source_path).expect_err("error expected");
    }

    #[test]
    /// Test activity file is corrupted
    fn test_activity_data_from_file_failure_file_empty() {
        // get the directory of the test executable
        let mut source_path = std::env::current_exe()
            .unwrap()
            .parent()
            .expect("executable directory")
            .to_path_buf();

        // go up to the repository base directory
        source_path.pop();
        source_path.pop();
        source_path.pop();

        // append location of the test data
        source_path.push("test");
        source_path.push("corrupted.fit");

        super::parse_fit_file(&source_path).expect_err("error expected");
    }
}
