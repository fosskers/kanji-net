use gumdrop::{Options, ParsingStyle};
use kanji::exam_lists::*;
use kn_core::{Entry, Error, KGraph, Kanji, Level, DB};
use petgraph::dot::{Config, Dot};
use petgraph::prelude::*;
use std::collections::HashSet;
use std::io::{self, Stdin, Stdout, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
    /// Show database statistics.
    Stats(Stats),
}

#[derive(Options)]
struct New {}

#[derive(Options)]
struct Graph {
    /// Show this help message.
    help: bool,

    /// Kanji whose families you wish to focus on.
    #[options(free, parse(try_from_str = "kanji_from_str"))]
    kanji: Option<Kanji>,
}

#[derive(Options)]
struct Stats {}

fn main() -> Result<(), Error> {
    let args = Args::parse_args_or_exit(ParsingStyle::AllOptions);

    match args.command {
        _ if args.version => {
            let version = env!("CARGO_PKG_VERSION");
            println!("{}", version);
            Ok(())
        }
        Some(Command::New(_)) => new_entry(&args.data),
        Some(Command::Graph(g)) => graph_dot(&args.data, g.kanji),
        Some(Command::Stats(_)) => db_stats(&args.data),
        None => Ok(()),
    }
}

fn new_entry(path: &Path) -> Result<(), Error> {
    let mut db = kn_core::open_db(path)?;
    let entry = kanji_prompt()?;
    let kanji = entry.kanji;

    match db.entries.insert(kanji, entry) {
        Some(_) => Err(Error::Exists(kanji)),
        None => kn_core::write_db(path, db),
    }
}

/// Prompt the user for the fields of an `Entry` to add to the database.
fn kanji_prompt() -> Result<Entry, Error> {
    let in_handle = io::stdin();
    let mut out_handle = io::stdout();

    let oya: Vec<Kanji> = get_line(&in_handle, &mut out_handle, "親")?
        .split_whitespace()
        .filter_map(|s| s.chars().next())
        .filter_map(Kanji::new)
        .collect();

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

fn graph_dot(path: &Path, mk: Option<Kanji>) -> Result<(), Error> {
    let db = kn_core::open_db(path)?;
    let graph = match mk {
        None => db.graph,
        Some(k) => filtered_graph(db, k)?,
    };
    println!("{}", graph_to_dot(&graph));
    Ok(())
}

fn graph_to_dot(graph: &KGraph) -> Dot<&KGraph> {
    Dot::with_attr_getters(
        graph,
        &[Config::EdgeNoLabel],
        &|_, e| e.weight().to_dot_attr(),
        &|_, _| "".to_string(),
    )
}

/// Hone in on specific Kanji families.
fn filtered_graph(db: DB, k: Kanji) -> Result<KGraph, Error> {
    let kix = db.index.get(&k).ok_or(Error::Missing(k))?;
    let children = all_children(&db, *kix);
    let parents = all_parents(&db, k);
    let indices: HashSet<NodeIndex<u16>> = children.union(&parents).map(|ix| *ix).collect();
    let filtered = db
        .graph
        .filter_map(|ix, k| indices.get(&ix).map(|_| *k), |_, e| Some(*e));

    Ok(filtered)
}

/// Walk down the graph to find all the descendants of the given `Kanji`.
fn all_children(db: &DB, kix: NodeIndex<u16>) -> HashSet<NodeIndex<u16>> {
    let mut ixs: HashSet<NodeIndex<u16>> = db
        .graph
        .neighbors_directed(kix, Direction::Outgoing)
        .flat_map(|kix| {
            let grandchildren = all_children(db, kix);
            let other_parents = db
                .graph
                .node_weight(kix)
                .and_then(|k| db.entries.get(k))
                .map(|e| {
                    e.oya
                        .iter()
                        .filter_map(|o| db.index.get(o))
                        .map(|ix| *ix)
                        .collect()
                })
                .unwrap_or_default();

            grandchildren
                .union(&other_parents)
                .map(|x| *x)
                .collect::<HashSet<_>>()
        })
        .collect();
    ixs.insert(kix);
    ixs
}

/// Walk up the graph to find all the ancestors of the given `Kanji`.
fn all_parents(db: &DB, k: Kanji) -> HashSet<NodeIndex<u16>> {
    db.entries
        .get(&k)
        .map(|e| {
            e.oya
                .iter()
                .filter_map(|o| {
                    let ix = db.index.get(o)?;
                    let mut parents = all_parents(db, *o);
                    parents.insert(*ix);
                    Some(parents)
                })
                .flatten()
                .collect()
        })
        .unwrap_or_else(|| HashSet::new())
}

fn kanji_from_str(s: &str) -> Result<Kanji, Error> {
    s.chars()
        .next()
        .and_then(Kanji::new)
        .ok_or(Error::NotKanji(s.to_string()))
}

fn db_stats(path: &Path) -> Result<(), Error> {
    let now = SystemTime::now();
    let db = kn_core::open_db(path)?;
    let micros = now.elapsed().map_err(Error::Time)?.as_micros();
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

    pairs.iter().for_each(|(level, len)| {
        let found = levels.iter().filter(|(_, l)| *l == level).count();
        println!("  - {:?}: {}/{}", level, found, len);
    });

    Ok(())
}
