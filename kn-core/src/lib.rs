//! Core types and functions for KanjiNet.

mod utils;

pub use kanji::Kanji;
use petgraph::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::path::Path;

/// The various errors that can occur while processing Kanji.
#[derive(Debug)]
pub enum Error {
    /// Some lower-level error involving file IO.
    IO(std::io::Error),
    /// Some lower-level involving JSON (de)serialization.
    JSON(serde_json::Error),
    /// A given `Kanji` already exists in the database.
    Exists(Kanji),
    /// A given `Kanji` is missing from the database.
    Missing(Kanji),
    /// The given `String` does not represent a single `Kanji`.
    NotKanji(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => e.fmt(f),
            Error::JSON(e) => e.fmt(f),
            Error::Exists(k) => write!(f, "{} already has an entry in the database.", k.get()),
            Error::Missing(k) => write!(f, "{} is missing from the database.", k.get()),
            Error::NotKanji(s) => write!(f, "{} is not Kanji.", s),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IO(e) => Some(e),
            Error::JSON(e) => Some(e),
            Error::Exists(_) => None,
            Error::Missing(_) => None,
            Error::NotKanji(_) => None,
        }
    }
}

/// The relationship between parents and children, in terms of their readings.
#[derive(Clone, Copy)]
pub enum Inherit {
    /// The child is the exact same as the parent. (e.g. こく→こく)
    Same,
    /// A secondary reading of the child is the same as the parent.
    Second,
    /// The child is a voicing variant of the parent. (e.g. こく→ごく)
    Voicing,
    /// The first consonant of the child is at least the same as the parent. (e.g. こく→けい)
    Consonant,
    /// The child bares no resemblance to the parent. (e.g. こく→よう)
    Differ,
    /// The child has no 音読み, which occurs often with 国字.
    None,
}

impl Inherit {
    // TODO These can be RGB! Make these nice pastels or something.
    pub fn to_dot_attr(&self) -> String {
        match self {
            Inherit::Same => "color=green".to_string(),
            Inherit::Second => "color=greenyellow".to_string(),
            Inherit::Voicing => "color=yellow".to_string(),
            Inherit::Consonant => "color=orange".to_string(),
            Inherit::Differ => "color=red".to_string(),
            Inherit::None => "color=gray".to_string(),
        }
    }
}

impl fmt::Display for Inherit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inherit::Same => write!(f, "Same"),
            Inherit::Second => write!(f, "Second"),
            Inherit::Voicing => write!(f, "Voicing"),
            Inherit::Consonant => write!(f, "Consonant"),
            Inherit::Differ => write!(f, "Differ"),
            Inherit::None => write!(f, "None"),
        }
    }
}

/// A convenient alias.
pub type KGraph = Graph<Kanji, Inherit, Directed, u16>;

/// An in-memory database for querying `Kanji` data.
pub struct DB {
    pub entries: HashMap<Kanji, Entry>,
    pub index: HashMap<Kanji, NodeIndex<u16>>,
    pub graph: KGraph,
}

impl DB {
    /// Create a new `DB` from a freshly read source of entries.
    ///
    /// # Panics
    ///
    /// Will panic if `Graph::add_node` panics, namely if the `HashMap` has over
    /// `u16` entries, which it never will.
    pub fn new(entries: HashMap<Kanji, Entry>) -> DB {
        let mut graph: KGraph = Graph::default();

        // Add all nodes to the graph.
        let index: HashMap<Kanji, NodeIndex<u16>> =
            entries.keys().map(|k| (*k, graph.add_node(*k))).collect();

        // Add all edges to the graph, where parents have directed edges to
        // their children.
        for e in entries.values() {
            // Safe unwrap, since we definitely added every `Kanji` key to the
            // `index` HashMap.
            let cix = index.get(&e.kanji).unwrap();
            e.oya
                .iter()
                .filter_map(|o| {
                    let oya = entries.get(o)?;
                    let oix = index.get(o)?;
                    Some((oya, oix, cix))
                })
                .for_each(|(oya, oix, cix)| {
                    let inherit = match (e.onyomi.get(0), oya.onyomi.get(0)) {
                        (Some(a), Some(b)) if a == b => Inherit::Same,
                        (Some(a), Some(b)) if utils::is_voiced_pair(a, b) => Inherit::Voicing,
                        (Some(_), Some(_))
                            if e.onyomi.iter().any(|a| oya.onyomi.iter().any(|b| a == b)) =>
                        {
                            Inherit::Second
                        }
                        (Some(_), Some(_)) => Inherit::Differ,
                        (_, _) => Inherit::None,
                    };
                    graph.add_edge(*oix, *cix, inherit);
                });
        }

        DB {
            entries,
            index,
            graph,
        }
    }
}

/// An entry in the kanji database.
#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub kanji: Kanji,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub oya: HashSet<Kanji>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub onyomi: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imi: Vec<(String, String)>,
}

/// Open a data file and bring the whole "database" into memory.
pub fn open_db(path: &Path) -> Result<DB, Error> {
    let raw = fs::read_to_string(path).map_err(Error::IO)?;
    let ks: Vec<Entry> = serde_json::from_str(&raw).map_err(Error::JSON)?;
    let hm = ks.into_iter().map(|e| (e.kanji, e)).collect();

    Ok(DB::new(hm))
}

/// Write a Kanji "database" into a file by order of its `Kanji`.
pub fn write_db(path: &Path, db: DB) -> Result<(), Error> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .map_err(Error::IO)?;
    let mut entries = db
        .entries
        .into_iter()
        .map(|(_, v)| v)
        .collect::<Vec<Entry>>();
    entries.sort_by_key(|e| e.kanji);
    serde_json::to_writer_pretty(file, &entries).map_err(Error::JSON)
}
