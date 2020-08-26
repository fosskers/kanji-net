//! Core types and functions for KanjiNet.

use kanji::Kanji;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// An entry in the kanji database.
#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub kanji: Kanji,
    pub oya: Vec<Kanji>,
    pub onyomi: Vec<String>,
    pub imi: Vec<(String, String)>,
}

/// Open a data file and bring the whole "database" into memory.
pub fn open_db(path: &Path) -> Result<HashMap<Kanji, Entry>, Error> {
    let raw = fs::read_to_string(path).map_err(Error::IO)?;
    let ks: Vec<Entry> = serde_json::from_str(&raw).map_err(Error::JSON)?;
    let mut hm = HashMap::new();

    ks.into_iter().for_each(|e| {
        hm.insert(e.kanji, e);
    });

    Ok(hm)
}

/// Write a Kanji "database" into a file by order of its `Kanji`.
pub fn write_db(path: &Path, mut hm: HashMap<Kanji, Entry>) -> Result<(), Error> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .map_err(Error::IO)?;
    let mut entries = hm.drain().map(|(_, v)| v).collect::<Vec<Entry>>();
    entries.sort_by_key(|e| e.kanji);
    serde_json::to_writer_pretty(file, &entries).map_err(Error::JSON)
}
