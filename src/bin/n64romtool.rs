use clap::{App, Arg, ArgMatches};
use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use std::process;
use failure::Fail;

use n64rom::bytes::Endianness;
use n64rom::io::Writer;
use n64rom::rom::Rom;
use n64rom::header;
use n64rom::util::{FileSize, MEBIBYTE};

#[derive(Debug, Fail)]
enum Error {
    /// Invalid CRC values.
    #[fail(display = "Bad CRC values, expected: (0x{:08X}, 0x{:08X})", _0, _1)]
    CRCError(u32, u32),

    /// Error parsing Header.
    #[fail(display = "{}", _0)]
    HeaderError(header::HeaderError),

    /// IO error.
    #[fail(display = "{}", _0)]
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
        .subcommand(
            App::new("correct")
                .about("Correct the CRC values of a rom file")
                .arg(Arg::with_name("file")
                    .required(true)
                    .help("Rom file"))
        )
        .get_matches();

    match main_with_args(&matches) {
        Ok(()) => Ok(()),
        Err(Error::HeaderError(err)) => {
            println!("Error: {}, are you sure this is a rom file?", err);
            process::exit(1);
        }
        Err(Error::CRCError(crc1, crc2)) => {
            // Display default CRCError message
            println!("{}", Error::CRCError(crc1, crc2));
            process::exit(1);
        }
        Err(err) => {
            println!("Error: {}", err);
            process::exit(1);
        }
    }
}

fn load_rom(path: &str, with_body: bool) -> Result<(Rom, File), Error> {
    let in_path = Path::new(path);
    let mut file = File::open(in_path)?;
    let rom = Rom::read_with_body(&mut file, with_body)?;
    Ok((rom, file))
}

fn load_rom_rw(path: &str) -> Result<(Rom, File), Error> {
    let in_path = Path::new(path);
    let mut file = OpenOptions::new().read(true).write(true).open(in_path)?;
    let rom = Rom::read(&mut file)?;
    Ok((rom, file))
}

fn main_with_args(matches: &ArgMatches) -> Result<(), Error> {

    match matches.subcommand() {
        ("check", Some(matches)) => {
            let path = matches.value_of("file").unwrap();
            let (rom, _) = load_rom(&path, true)?;

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
            let (rom, _) = load_rom(&path, true)?;

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
        ("correct", Some(matches)) => {
            let path = matches.value_of("file").unwrap();
            let (mut rom, mut file) = load_rom_rw(&path)?;

            if rom.correct_crc() {
                println!("Rom CRC values are already correct!");
                Ok(())
            } else {
                file.seek(SeekFrom::Start(0))?;

                // Use a writer that respects the original byte order
                let mut writer = Writer::from(&mut file, &rom.order());
                rom.header.write(&mut writer)?;
                writer.flush()?;

                println!("Corrected!");
                Ok(())
            }
        }
        ("show", Some(matches)) => {
            // Read rom with only head (header & IPL3)
            let path = matches.value_of("file").unwrap();
            let (rom, file) = load_rom(&path, false)?;

            // For efficiency, instead of reading all data to determine rom size, check file metadata
            let metadata = file.metadata()?;
            let filesize = FileSize::from(metadata.len(), MEBIBYTE);

            // Show size text in MiB
            let sizetext = match filesize {
                FileSize::Float(value) => {
                    format!("{:.*} MiB", 1, value)
                }
                FileSize::Int(value) => {
                    format!("{} MiB", value)
                }
            };

            println!("{}", rom);
            println!("  Rom Size: {}", &sizetext);

            Ok(())
        }
        ("", None) => {
            println!("No subcommand was used");
            Ok(())
        }
        _ => unreachable!(),
    }
}
