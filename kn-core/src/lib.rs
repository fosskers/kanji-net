//! Core types and functions for KanjiNet.

/// An entry in the kanji database.
struct Kanji {
    kanji: char,
    oya: Vec<char>,
    onyomi: Vec<String>,
    imi: Vec<(String, String)>,
}
