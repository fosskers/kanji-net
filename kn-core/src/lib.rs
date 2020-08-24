//! Core types and functions for KanjiNet.

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use std::str;

/// Wrapper around a library type so that we can give it additional trait
/// implementations.
pub struct Kanji(pub kanji::Kanji);

impl FromSql for Kanji {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(bytes) => from_bytes(bytes).ok_or(FromSqlError::InvalidType),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

fn from_bytes(bytes: &[u8]) -> Option<Kanji> {
    str::from_utf8(bytes)
        .ok()?
        .chars()
        .next()
        .and_then(kanji::Kanji::new)
        .map(Kanji)
}

/// An entry in the kanji database.
pub struct Entry {
    pub kanji: Kanji,
    pub oya: Vec<Kanji>,
    pub onyomi: Vec<String>,
    pub imi: Vec<(String, String)>,
}
