use clap::{App, Arg};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use n64rom::header::N64Header;

fn main() -> Result<(), Box<Error>> {
    let matches = App::new("n64romtool")
        .author("saneki <s@neki.me>")
        .about("Displays information about N64 ROM files")
        .arg(Arg::with_name("FILE")
            .required(true))
        .get_matches();

    let in_path = Path::new(matches.value_of("FILE").unwrap());
    let header = {
        let mut file = File::open(in_path)?;
        N64Header::read(&mut file)
    };

    match header {
        Ok(header) => println!("{}", header),
        Err(e) => println!("Error reading file: {}", e)
    }

    Ok(())
}
