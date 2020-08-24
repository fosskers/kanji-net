//! Core types and functions for KanjiNet.

use kanji::Kanji;

/// An entry in the kanji database.
struct Entry {
    kanji: Kanji,
    oya: Vec<Kanji>,
    onyomi: Vec<String>,
    imi: Vec<(String, String)>,
}
