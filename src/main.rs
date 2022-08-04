use std::fs::File;

fn main() {
    println!(
        "Parsing FIT files using Profile version: {}",
        fitparser::profile::VERSION
    );

    let mut fp = match File::open("/tmp/activity.fit") {
        Ok(fp) => fp,
        Err(err) => {
            println!("Error: {}", err);
            std::process::exit(1);
        }
    };

    let alldata = match fitparser::from_reader(&mut fp) {
        Ok(alldata) => alldata,
        Err(err) => {
            println!("Error: {}", err);
            std::process::exit(1);
        }
    };

    for data in alldata {
        if fitparser::profile::field_types::MesgNum::FileId == data.kind() {
            println!("=> {}", data.kind());
            for field in data.fields() {
                if field.name().eq("time_created") {
                    println!("==> {} {:#?}", field.name(), field.value());
                }
            }
        }
        if fitparser::profile::field_types::MesgNum::Sport == data.kind() {
            println!("=> {}", data.kind());
            for field in data.fields() {
                if field.name().eq("sport") {
                    println!("==> {} {}", field.name(), field.value());
                }
            }
        }
    }
}
