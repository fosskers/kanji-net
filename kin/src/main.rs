use gumdrop::{Options, ParsingStyle};
use kanji::exam_lists::*;
use kn_core::{self as core, DotMode, Entry, Kanji, Level};
use rustyline::history::{FileHistory, History};
use rustyline::Editor;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Options)]
struct Args {
    /// Show this help message.
    help: bool,
    /// Show the current version of `kin`.
    version: bool,
    /// Path to the Kanji data file.
    #[options(meta = "PATH", default = "/home/colin/code/rust/kanji-net/data.json")]
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
    /// Show database statistics.
    Stats(Stats),
    /// Show the levels of given Kanji.
    Levels(Levels),
    /// Give the next Kanji yet unentered into the DB.
    Next(Next),
}

#[derive(Options)]
struct New {}

#[derive(Options)]
struct Graph {
    /// Show this help message.
    help: bool,
    /// Search via the given Kanji's parents, not the Kanji itself.
    parents: bool,
    /// Filepath to write the image to.
    #[options(meta = "PATH", default = "graph.png")]
    output: PathBuf,
    /// Kanji whose families you wish to focus on.
    #[options(free, parse(from_str = "kanji_from_str"))]
    kanji: Vec<Vec<Kanji>>,
}

/// Various statistics about the Kanji database.
#[derive(Options)]
struct Stats {}

/// Inspect the levels of given Kanji.
#[derive(Options)]
struct Levels {
    /// Show this help message.
    help: bool,
    /// Kanji whose level you wish to inspect.
    #[options(free)]
    kanji: Vec<String>,
}

#[derive(Options)]
struct Next {}

#[derive(Debug)]
enum Error {
    Core(core::Error),
    Readline(rustyline::error::ReadlineError),
}

impl From<core::Error> for Error {
    fn from(v: core::Error) -> Self {
        Self::Core(v)
    }
}

fn main() -> Result<(), Error> {
    let args = Args::parse_args_or_exit(ParsingStyle::AllOptions);

    match args.command {
        _ if args.version => {
            let version = env!("CARGO_PKG_VERSION");
            println!("{}", version);
        }
        Some(Command::New(_)) => new_entry(&args.data)?,
        Some(Command::Graph(g)) => graph_dot(&args.data, g)?,
        Some(Command::Stats(_)) => db_stats(&args.data)?,
        Some(Command::Levels(l)) => levels(l.kanji),
        Some(Command::Next(_)) => next(&args.data)?,
        None => {}
    }

    Ok(())
}

fn new_entry(path: &Path) -> Result<(), Error> {
    let mut db = kn_core::open_db(path)?;
    let entry = kanji_prompt()?;
    let kanji = entry.kanji;

    // On collision, the entry is put into the in-memory copy of the DB, but
    // never makes it to the on-disk version.
    match db.entries.insert(kanji, entry) {
        Some(_) => Err(core::Error::Exists(kanji))?,
        None => kn_core::write_db(path, db)?,
    }

    Ok(())
}

/// Prompt the user for the fields of an `Entry` to add to the database.
fn kanji_prompt() -> Result<Entry, Error> {
    let mut rl = Editor::<(), FileHistory>::new().map_err(Error::Readline)?;
    rl.load_history("history.txt").map_err(Error::Readline)?;

    let oya: Vec<Kanji> = get_line(&mut rl, "親: ")?
        .split_whitespace()
        .flat_map(|s| s.chars())
        .filter_map(Kanji::new)
        .collect();

    let kakushi_oya: Vec<Kanji> = get_line(&mut rl, "隠し親: ")?
        .split_whitespace()
        .flat_map(|s| s.chars())
        .filter_map(Kanji::new)
        .collect();

    let kanji = get_legal_kanji(&mut rl, "漢字: ")?;

    let onyomi = get_line(&mut rl, "音読み: ")?
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let daihyou: Vec<String> = get_line(&mut rl, "代表: ")?
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let entry = Entry {
        kanji,
        oya,
        kakushi_oya,
        onyomi,
        daihyou,
    };

    rl.save_history("history.txt").map_err(Error::Readline)?;

    Ok(entry)
}

fn get_line<H>(rl: &mut Editor<(), H>, label: &str) -> Result<String, Error>
where
    H: History,
{
    match rl.readline(label) {
        Ok(line) => {
            rl.add_history_entry(&line).map_err(Error::Readline)?;
            Ok(line)
        }
        Err(_) => Err(core::Error::Other("CLI input failed.".to_string()))?,
    }
}

/// Loop on the input of legal Kanji.
fn get_legal_kanji<H>(rl: &mut Editor<(), H>, label: &str) -> Result<Kanji, Error>
where
    H: History,
{
    let line = get_line(rl, label)?;
    let mut chars = line.chars();

    match chars.next().and_then(Kanji::new) {
        Some(k) => Ok(k),
        _ => {
            println!("Invalid input! Try again.");
            get_legal_kanji(rl, label)
        }
    }
}

// FIXME This should use `NESet` from the get-go.
fn graph_dot(path: &Path, g: Graph) -> Result<(), core::Error> {
    let ks: Vec<Kanji> = g.kanji.into_iter().flatten().collect();
    let db = kn_core::open_db(path)?;

    // Note: This demonstrates how to do shell piping from within Rust.
    let mut child = std::process::Command::new("dot")
        .arg("-Tpng")
        .arg("-o")
        .arg(g.output)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let dot = if ks.is_empty() {
        db.dot()
    } else {
        // The kanji we should specially highlight in the final graph.
        let highlight_by: HashSet<Kanji> = ks.iter().copied().collect();

        // The kanji by which we filter the graph down.
        let hone_by = if g.parents {
            ks.iter()
                .filter_map(|k| db.entries.get(k).map(|e| e.oya.iter()))
                .flatten()
                .copied()
                .collect()
        } else {
            ks
        };

        db.dot_custom(DotMode::Groups, highlight_by, &db.filtered_graph(hone_by))
    };

    // Ensures that the handle to `stdin` drops and closes, avoiding a deadlock.
    // The `unwrap` here is on purpose and advertised in the definition of `stdin` itself.
    {
        let mut stdin: std::process::ChildStdin = child.stdin.take().unwrap();
        writeln!(stdin, "{}", dot)?; // FIXME Write the bytes directly?
    }

    match child.wait() {
        Err(e) => Err(core::Error::from(e)),
        Ok(_) => Ok(()),
    }
}

fn kanji_from_str(s: &str) -> Vec<Kanji> {
    s.chars().filter_map(Kanji::new).collect()
}

fn db_stats(path: &Path) -> Result<(), core::Error> {
    let now = SystemTime::now();
    let db = kn_core::open_db(path)?;
    let micros = now.elapsed().map_err(core::Error::Time)?.as_micros();
    let levels = db.levels();

    let pairs = vec![
        (Level::Ten, LEVEL_10.chars().count()),
        (Level::Nine, LEVEL_09.chars().count()),
        (Level::Eight, LEVEL_08.chars().count()),
        (Level::Seven, LEVEL_07.chars().count()),
        (Level::Six, LEVEL_06.chars().count()),
        (Level::Five, LEVEL_05.chars().count()),
        (Level::Four, LEVEL_04.chars().count()),
        (Level::Three, LEVEL_03.chars().count()),
        (Level::PreTwo, LEVEL_02_PRE.chars().count()),
        (Level::Two, LEVEL_02.chars().count()),
        (Level::PreOne, LEVEL_01_PRE.chars().count()),
        (Level::One, LEVEL_01.chars().count()),
    ];

    println!("DB loaded in {} microseconds.", micros);
    println!("DB contains {} entries.", db.entries.len());
    println!("Kanji Levels completed:");

    let jouyou: usize = pairs.iter().take(10).map(|(_, c)| c).sum();
    let level_totals: HashMap<Level, usize> = pairs
        .iter()
        .map(|(level, _)| {
            let found = levels.iter().filter(|(_, l)| *l == level).count();
            (*level, found)
        })
        .collect();

    pairs.iter().for_each(|(level, len)| {
        let found = level_totals.get(level).unwrap();
        println!("  - {:?}: {}/{}", level, found, len);
    });

    let entered_jouyou: usize = level_totals
        .iter()
        .filter(|(&l, _)| l < Level::PreOne)
        .map(|(_, s)| s)
        .sum();
    let percent = 100.0 * (entered_jouyou as f64) / (jouyou as f64);
    println!(
        "常用 Completion: {}/{} ({:.2}%)",
        entered_jouyou, jouyou, percent
    );

    Ok(())
}

fn levels(ks: Vec<String>) {
    let table = kanji::level_table();

    ks.iter()
        .flat_map(|s| s.chars())
        .filter_map(Kanji::new)
        .for_each(|k| {
            table
                .get(&k)
                .into_iter()
                .for_each(|l| println!("{}: {:?}", k, l))
        })
}

fn next(path: &Path) -> Result<(), core::Error> {
    let db = kn_core::open_db(path)?;

    LEVEL_10
        .chars()
        .chain(LEVEL_09.chars())
        .chain(LEVEL_08.chars())
        .chain(LEVEL_07.chars())
        .chain(LEVEL_06.chars())
        .chain(LEVEL_05.chars())
        .chain(LEVEL_04.chars())
        .chain(LEVEL_03.chars())
        .chain(LEVEL_02_PRE.chars())
        .chain(LEVEL_02.chars())
        .chain(LEVEL_01_PRE.chars())
        .chain(LEVEL_01.chars())
        .filter_map(kanji::Kanji::new)
        .find(|k| db.entries.get(k).is_none())
        .iter()
        .for_each(|k| println!("{}", k));

    Ok(())
}
