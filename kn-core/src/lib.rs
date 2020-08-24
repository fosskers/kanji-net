//! Core types and functions for KanjiNet.

/// Wrapper around a library type so that we can give it additional trait
/// implementations.
pub struct Kanji(pub kanji::Kanji);

// TODO Give `Kanji` serde instances via a feature flag.

/// An entry in the kanji database.
pub struct Entry {
    pub kanji: Kanji,
    pub oya: Vec<Kanji>,
    pub onyomi: Vec<String>,
    pub imi: Vec<(String, String)>,
}
