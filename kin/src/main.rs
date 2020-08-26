use gumdrop::{Options, ParsingStyle};
use kn_core::{Entry, Error, Kanji};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Options)]
struct Args {
    /// Show this help message.
    help: bool,

    /// Show the current version of `kin`.
    version: bool,

    /// Path to the Kanji data file.
    #[options(meta = "PATH", default = "data.json")]
    data: PathBuf,
}

fn main() -> Result<(), Error> {
    let args = Args::parse_args_or_exit(ParsingStyle::AllOptions);

    if args.version {
        let version = env!("CARGO_PKG_VERSION");
        println!("{}", version);
        Ok(())
    } else {
        work(&args.data)
    }
}

fn work(path: &Path) -> Result<(), Error> {
    let mut data = kn_core::open_db(path)?;
    let entry = kanji_prompt()?;
    let kanji = entry.kanji;

    match data.insert(kanji, entry) {
        Some(_) => Err(Error::Exists(kanji)),
        None => kn_core::write_db(path, data),
    }
}

/// Prompt the user for the fields of an `Entry` to add to the database.
fn kanji_prompt() -> Result<Entry, Error> {
    let in_handle = io::stdin();
    let mut out_handle = io::stdout();
    let mut line = String::new();
    print!("Kanji: ");
    out_handle.flush().map_err(Error::IO)?;
    in_handle.read_line(&mut line).map_err(Error::IO)?;

    let kanji = Kanji::new(line.chars().next().unwrap()).unwrap();
    let entry = Entry {
        kanji,
        oya: vec![],
        onyomi: vec![],
        imi: vec![],
    };

    Ok(entry)
}
