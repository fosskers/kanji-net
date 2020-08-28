use gumdrop::{Options, ParsingStyle};
use kn_core::{Entry, Error, Kanji, DB};
use petgraph::dot::{Config, Dot};
use std::collections::HashSet;
use std::io::{self, Stdin, Stdout, Write};
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

    #[options(command)]
    command: Option<Command>,
}

#[derive(Options)]
enum Command {
    /// Add a new entry to the database.
    New(New),
    /// Output the content of the Kanji Graph in Dot format.
    Graph(Graph),
}

#[derive(Options)]
struct New {}

#[derive(Options)]
struct Graph {}

fn main() -> Result<(), Error> {
    let args = Args::parse_args_or_exit(ParsingStyle::AllOptions);

    match args.command {
        _ if args.version => {
            let version = env!("CARGO_PKG_VERSION");
            println!("{}", version);
            Ok(())
        }
        Some(Command::New(_)) => new_entry(&args.data),
        Some(Command::Graph(_)) => graph_dot(&args.data),
        None => Ok(()),
    }
}

fn new_entry(path: &Path) -> Result<(), Error> {
    let mut db = kn_core::open_db(path)?;
    let entry = kanji_prompt(&db)?;
    let kanji = entry.kanji;

    match db.entries.insert(kanji, entry) {
        Some(_) => Err(Error::Exists(kanji)),
        None => kn_core::write_db(path, db),
    }
}

/// Prompt the user for the fields of an `Entry` to add to the database.
fn kanji_prompt(db: &DB) -> Result<Entry, Error> {
    let in_handle = io::stdin();
    let mut out_handle = io::stdout();

    let oya: HashSet<Kanji> = get_line(&in_handle, &mut out_handle, "親")?
        .split_whitespace()
        .filter_map(|s| s.chars().next())
        .filter_map(Kanji::new)
        .collect();

    // let children: String = data
    //     .values()
    //     .filter(|v| !v.oya.is_disjoint(&oya))
    //     .map(|v| v.kanji.get())
    //     .collect();

    // if !children.is_empty() {
    //     println!("Children of those parents:");
    //     println!("  {}", children);
    // }

    let kanji = get_legal_kanji(&in_handle, &mut out_handle, "漢字")?;
    let onyomi = get_line(&in_handle, &mut out_handle, "音読み")?
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let entry = Entry {
        kanji,
        oya,
        onyomi,
        imi: vec![],
    };

    Ok(entry)
}

fn get_line(in_handle: &Stdin, out_handle: &mut Stdout, label: &str) -> Result<String, Error> {
    let mut line = String::new();
    print!("{}: ", label);
    out_handle.flush().map_err(Error::IO)?;
    in_handle.read_line(&mut line).map_err(Error::IO)?;
    Ok(line.trim_end().to_string())
}

/// Loop on the input of legal Kanji.
fn get_legal_kanji(
    in_handle: &Stdin,
    out_handle: &mut Stdout,
    label: &str,
) -> Result<Kanji, Error> {
    let line = get_line(in_handle, out_handle, label)?;
    let mut chars = line.chars();

    match chars.next().and_then(Kanji::new) {
        Some(k) => Ok(k),
        _ => {
            println!("Invalid input! Try again.");
            get_legal_kanji(in_handle, out_handle, label)
        }
    }
}

fn graph_dot(path: &Path) -> Result<(), Error> {
    let db = kn_core::open_db(path)?;
    println!("{:?}", Dot::with_config(&db.graph, &[Config::EdgeNoLabel]));
    Ok(())
}
