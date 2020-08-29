//! Core types and functions for KanjiNet.

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
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => e.fmt(f),
            Error::JSON(e) => e.fmt(f),
            Error::Exists(k) => write!(f, "{} already has an entry in the database.", k.get()),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IO(e) => Some(e),
            Error::JSON(e) => Some(e),
            Error::Exists(_) => None,
        }
    }
}

/// The relationship between parents and children, in terms of their readings.
pub enum Inheritance {
    /// The child is the exact same as the parent. (e.g. こく→こく)
    Same,
    /// A secondary reading of the child is the same as the parent.
    Secondary,
    /// The child is a voicing variant of the parent. (e.g. こく→ごく)
    Voicing,
    /// The first consonant of the child is at least the same as the parent. (e.g. こく→けい)
    Consonant,
    /// The child rhymes with the the parent. (e.g. こく→よく)
    Rhymes,
    /// The child bares no resemblance to the parent. (e.g. こく→よう)
    Differ,
}

impl fmt::Display for Inheritance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Inheritance::Same => write!(f, "Same"),
            Inheritance::Secondary => write!(f, "Secondary"),
            Inheritance::Voicing => write!(f, "Voicing"),
            Inheritance::Consonant => write!(f, "Consonant"),
            Inheritance::Rhymes => write!(f, "Rhymes"),
            Inheritance::Differ => write!(f, "Differ"),
        }
    }
}

/// An in-memory database for querying `Kanji` data.
pub struct DB {
    pub entries: HashMap<Kanji, Entry>,
    pub index: HashMap<Kanji, NodeIndex<u16>>,
    pub graph: Graph<Kanji, Inheritance, Directed, u16>,
}

impl DB {
    /// Create a new `DB` from a freshly read source of entries.
    ///
    /// # Panics
    ///
    /// Will panic if `Graph::add_node` panics, namely if the `HashMap` has over
    /// `u16` entries, which it never will.
    pub fn new(entries: HashMap<Kanji, Entry>) -> DB {
        let mut graph: Graph<Kanji, Inheritance, Directed, u16> = Graph::default();

        // Add all nodes to the graph.
        let index: HashMap<Kanji, NodeIndex<u16>> =
            entries.keys().map(|k| (*k, graph.add_node(*k))).collect();

        // Add all edges to the graph, where parents have directed edges to
        // their children.
        for e in entries.values() {
            // Safe unwrap, since we definitely added every `Kanji` key to the
            // `index` HashMap.
            let child = index.get(&e.kanji).unwrap();
            e.oya
                .iter()
                .filter_map(|o| {
                    let oya = index.get(o)?;
                    Some((oya, child))
                })
                .for_each(|(o, c)| {
                    graph.add_edge(*o, *c, Inheritance::Differ); // TODO Make proper inheritance.
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
