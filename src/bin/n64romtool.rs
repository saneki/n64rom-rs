use clap::{App, Arg};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use n64rom::rom::Rom;

fn main() -> Result<(), Box<Error>> {
    let matches = App::new("n64romtool")
        .author("saneki <s@neki.me>")
        .about("Displays information about N64 ROM files")
        .arg(Arg::with_name("FILE")
            .required(true))
        .get_matches();

    let in_path = Path::new(matches.value_of("FILE").unwrap());
    let rom = {
        let mut file = File::open(in_path)?;
        Rom::read(&mut file)
    };

    match rom {
        Ok(rom) => println!("{}", rom),
        Err(e) => println!("Error reading file: {}", e)
    }

    Ok(())
}
