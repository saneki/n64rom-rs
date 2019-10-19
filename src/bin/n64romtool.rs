use clap::{App, Arg, ArgMatches};
use std::fs::File;
use std::io;
use std::path::Path;
use failure::Fail;

use n64rom::bytes::Endianness;
use n64rom::rom::Rom;
use n64rom::header;

#[derive(Debug, Fail)]
enum Error {
    /// Invalid CRC values.
    #[fail(display = "Bad CRC values, expected {:08X}, {:08X}", _0, _1)]
    CRCError(u32, u32),

    /// Error parsing Header.
    #[fail(display = "Header Error")]
    HeaderError(header::HeaderError),

    /// IO error.
    #[fail(display = "IO Error")]
    IOError(io::Error),
}

impl From<header::HeaderError> for Error {
    fn from(e: header::HeaderError) -> Self {
        Error::HeaderError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IOError(e)
    }
}

fn main() -> Result<(), Error> {
    let matches = App::new("n64romtool")
        .author("saneki <s@neki.me>")
        .about("Displays information about N64 ROM files")
        .subcommand(
            App::new("show")
                .about("Show details about a rom file")
                .arg(Arg::with_name("file")
                    .required(true)
                    .help("Rom file"))
        )
        .subcommand(
            App::new("check")
                .about("Verify whether or not the CRC values of a rom file are correct")
                .arg(Arg::with_name("file")
                    .required(true)
                    .help("Rom file"))
        )
        .subcommand(
            App::new("convert")
                .about("Convert a rom file to a different byte order")
                .arg(Arg::with_name("order")
                    .takes_value(true)
                    .possible_values(&["big", "little", "mixed"])
                    .required(true)
                    .help("Byte order to convert to"))
                .arg(Arg::with_name("input")
                    .required(true)
                    .help("Input rom file"))
                .arg(Arg::with_name("output")
                    .required(true)
                    .help("Output rom file"))
        )
        .get_matches();

    main_with_args(&matches)
}

fn load_rom(path: &str) -> Result<Rom, Error> {
    let in_path = Path::new(path);
    let rom = {
        let mut file = File::open(in_path)?;
        Rom::read(&mut file)?
    };

    Ok(rom)
}

fn main_with_args(matches: &ArgMatches) -> Result<(), Error> {

    match matches.subcommand() {
        ("check", Some(matches)) => {
            let path = matches.value_of("file").unwrap();
            let rom = load_rom(&path)?;

            let (result, crcs) = rom.check_crc();
            if result {
                println!("Correct!");
                Ok(())
            } else {
                Err(Error::CRCError(crcs.0, crcs.1))
            }
        }
        ("convert", Some(matches)) => {
            let path = matches.value_of("input").unwrap();
            let rom = load_rom(&path)?;

            let order = match matches.value_of("order").unwrap() {
                "big" => Endianness::Big,
                "little" => Endianness::Little,
                "mixed" => Endianness::Mixed,
                _ => unreachable!(),
            };

            // Check if the rom file is already in this byte order
            if rom.order() == &order {
                println!("Rom file is already in {} byte order.", order);
                return Ok(());
            }

            let out_path = matches.value_of("output").unwrap();
            let mut file = File::create(out_path)?;
            rom.write(&mut file, Some(&order))?;
            println!("Done!");
            Ok(())
        }
        ("show", Some(matches)) => {
            let path = matches.value_of("file").unwrap();
            let rom = load_rom(&path)?;

            println!("{}", rom);
            Ok(())
        }
        ("", None) => {
            println!("No subcommand was used");
            Ok(())
        }
        _ => unreachable!(),
    }
}
