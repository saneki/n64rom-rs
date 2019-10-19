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
        .arg(Arg::with_name("check-crc")
            .short("c")
            .long("check-crc")
            .help("Verifies whether or not the CRC values are correct"))
        .arg(Arg::with_name("convert-order")
            .short("w")
            .long("convert-order")
            .takes_value(true)
            .possible_values(&["big", "little", "mixed"])
            .requires("output")
            .help("Byte order to convert to"))
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .takes_value(true)
            .help("Output file"))
        .arg(Arg::with_name("FILE")
            .required(true))
        .get_matches();

    main_with_args(&matches)
}

fn main_with_args(matches: &ArgMatches) -> Result<(), Error> {
    let in_path = Path::new(matches.value_of("FILE").unwrap());
    let rom = {
        let mut file = File::open(in_path)?;
        Rom::read(&mut file)?
    };

    main_with_rom(&rom, &matches)
}

fn main_with_rom(rom: &Rom, matches: &ArgMatches) -> Result<(), Error> {
    if matches.is_present("check-crc") {
        let (result, crcs) = rom.check_crc();
        if result {
            println!("Correct!");
        } else {
            return Err(Error::CRCError(crcs.0, crcs.1));
        }
    } else if matches.is_present("convert-order") {
        let order = match matches.value_of("convert-order").unwrap() {
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
        println!("Done!")
    } else {
        println!("{}", rom);
    }

    Ok(())
}
