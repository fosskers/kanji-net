//! Core types and functions for KanjiNet.

use kanji::Kanji;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::path::Path;

/// An entry in the kanji database.
#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub kanji: Kanji,
    pub oya: Vec<Kanji>,
    pub onyomi: Vec<String>,
    pub imi: Vec<(String, String)>,
}

/// Open a data file and bring the whole "database" into memory.
pub fn open_db(path: &Path) -> Option<HashMap<Kanji, Entry>> {
    let raw = fs::read_to_string(path).ok()?;
    let ks: Vec<Entry> = serde_json::from_str(&raw).ok()?;
    let mut hm = HashMap::new();

    ks.into_iter().for_each(|e| {
        hm.insert(e.kanji, e);
    });

    Some(hm)
}

pub fn write_db(path: &Path, mut hm: HashMap<Kanji, Entry>) -> Option<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .ok()?;
    let mut entries = hm.drain().map(|(_, v)| v).collect::<Vec<Entry>>();
    entries.sort_by_key(|e| e.kanji);
    serde_json::to_writer_pretty(file, &entries).ok()?;

    Some(())
}
