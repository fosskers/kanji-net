//! Core types and functions for KanjiNet.

use kanji::Kanji;
use serde::{Deserialize, Serialize};

/// An entry in the kanji database.
#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub kanji: Kanji,
    pub oya: Vec<Kanji>,
    pub onyomi: Vec<String>,
    pub imi: Vec<(String, String)>,
}
