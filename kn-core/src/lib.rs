//! Core types and functions for KanjiNet.

mod utils;

use itertools::Itertools;
pub use kanji::{Kanji, Level};
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
    Io(std::io::Error),
    /// Some lower-level error involving JSON (de)serialization.
    Json(serde_json::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::Json(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Json(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
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
    /// The child is a rhyme of the parent. (e.g. こく→よく)
    Rhyme,
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
            Inherit::Rhyme => "color=yellow".to_string(), // TODO Consider different colour.
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
            Inherit::Rhyme => write!(f, "Rhyme"),
            Inherit::Consonant => write!(f, "Consonant"),
            Inherit::Differ => write!(f, "Differ"),
            Inherit::None => write!(f, "None"),
        }
    }
}

/// A convenient alias.
pub type KGraph = Graph<Kanji, Inherit, Directed, u16>;

/// Specific settings for producing the Dot graph.
pub enum DotMode {
    NoGroups,
    Groups,
}

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
                    let inherit = match (e.onyomi.first(), oya.onyomi.first()) {
                        (Some(a), Some(b)) if a == b => Inherit::Same,
                        (Some(a), Some(b)) if utils::is_voiced_pair(a, b) => Inherit::Voicing,
                        (Some(a), Some(b)) if utils::is_rhyme(a, b) => Inherit::Rhyme,
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

    /// The full `Entry` associated with some index.
    pub fn entry(&self, nix: NodeIndex<u16>) -> Option<&Entry> {
        self.graph
            .node_weight(nix)
            .and_then(|k| self.entries.get(k))
    }

    /// Fetch the Exam levels of all `Kanji` in the database.
    pub fn levels(&self) -> HashMap<Kanji, Level> {
        let table = kanji::level_table();
        self.entries
            .iter()
            .filter_map(|(k, _)| table.get(k).map(|l| (*k, *l)))
            .collect()
    }

    /// Custom DOT output for a `KGraph`.
    pub fn dot(&self) -> String {
        self.dot_custom(DotMode::NoGroups, HashSet::new(), &self.graph)
    }

    /// Same as `dot`, but supply your own graph to consider.
    pub fn dot_custom(&self, dot_mode: DotMode, chosen: HashSet<Kanji>, graph: &KGraph) -> String {
        let levels = kanji::level_table();
        let mut s = String::new();
        s.push_str("digraph {\n");

        let filtered = graph.node_indices().filter_map(|kix| {
            graph
                .node_weight(kix)
                .and_then(|k| self.entries.get(k))
                .map(|e| (kix, e.kanji, e.onyomi.first(), levels.get(&e.kanji)))
        });

        match dot_mode {
            DotMode::Groups => DB::with_groups(&chosen, &mut s, filtered),
            DotMode::NoGroups => filtered.for_each(|(kix, k, _, _)| {
                let shape = DB::shape(&chosen, &k);
                let line = format!("    {} [ label=\"{}\", shape={} ]\n", kix.index(), k, shape);
                s.push_str(&line);
            }),
        }

        // Gap between nodes and edges.
        s.push('\n');

        // Write all the edges.
        graph.raw_edges().iter().for_each(|e| {
            let line = format!(
                "    {} -> {} [ {} ]\n",
                e.source().index(),
                e.target().index(),
                e.weight.to_dot_attr(),
            );
            s.push_str(&line);
        });

        s.push_str("}\n");
        s
    }

    fn shape(chosen: &HashSet<Kanji>, k: &Kanji) -> &'static str {
        if chosen.contains(k) {
            "doublecircle"
        } else {
            "circle"
        }
    }

    fn with_groups<'a, F>(chosen: &HashSet<Kanji>, s: &mut String, filtered: F)
    where
        F: Iterator<Item = (NodeIndex<u16>, Kanji, Option<&'a String>, Option<&'a Level>)>,
    {
        filtered
            .sorted_by(|a, b| a.2.cmp(&b.2))
            .group_by(|pair| pair.2)
            .into_iter()
            .for_each(|(yomi, group)| {
                // An unfortunate `collect` to know the number of elements with certainty.
                let g: Vec<_> = group.collect();

                match yomi {
                    // Only bother grouping if there is more than one node in the group.
                    Some(y) if g.len() > 1 => {
                        s.push('\n');
                        s.push_str(&format!("    subgraph cluster_{} {{\n", y));
                        s.push_str(&format!("        label=\"{}\";\n", y));
                        s.push_str("        style=dashed;\n");
                        s.push_str("        color=brown;\n");
                        s.push('\n');
                        g.into_iter().for_each(|(kix, k, _, l)| {
                            let shape = DB::shape(chosen, &k);
                            let line = format!(
                                "        {} [ label=<{}{}>, shape={} ];\n",
                                kix.index(),
                                k,
                                l.map(|x| format!("<br/><font point-size=\"8\">{}</font>", x))
                                    .unwrap_or_else(|| "".to_string()),
                                shape
                            );
                            s.push_str(&line);
                        });
                        s.push_str("    }\n\n");
                    }
                    _ => g.into_iter().for_each(|(kix, k, _, l)| {
                        let shape = DB::shape(chosen, &k);
                        let line = format!(
                            "    {} [ label=<{}{}>, shape={} ]\n",
                            kix.index(),
                            k,
                            l.map(|x| format!("<br/><font point-size=\"8\">{}</font>", x))
                                .unwrap_or_else(|| "".to_string()),
                            shape
                        );
                        s.push_str(&line);
                    }),
                }
            })
    }

    /// Hone in on specific Kanji families.
    pub fn filtered_graph(&self, ks: Vec<Kanji>) -> KGraph {
        let children: HashSet<_> = ks
            .iter()
            .filter_map(|k| self.index.get(k))
            .flat_map(|kix| self.all_children(*kix))
            .collect();
        let parents: HashSet<_> = ks.into_iter().flat_map(|k| self.all_parents(k)).collect();
        let indices: HashSet<NodeIndex<u16>> = children.union(&parents).copied().collect();

        self.graph
            .filter_map(|ix, k| indices.get(&ix).map(|_| *k), |_, e| Some(*e))
    }

    /// Walk down the graph to find all the descendants of the given `Kanji`.
    fn all_children(&self, kix: NodeIndex<u16>) -> HashSet<NodeIndex<u16>> {
        let mut ixs: HashSet<NodeIndex<u16>> = self
            .graph
            .neighbors_directed(kix, Direction::Outgoing)
            .flat_map(|kix| {
                let grandchildren = self.all_children(kix);
                let other_parents: HashSet<_> = self
                    .entry(kix)
                    .map(|e| {
                        e.oya
                            .iter()
                            .filter_map(|o| self.index.get(o).map(|ix| (o, ix)))
                            .flat_map(|(o, ix)| {
                                let mut others = self.all_parents(*o);
                                others.insert(*ix);
                                others
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                grandchildren
                    .union(&other_parents)
                    .copied()
                    .collect::<HashSet<_>>()
            })
            .collect();
        ixs.insert(kix);
        ixs
    }

    /// Walk up the graph to find all the ancestors of the given `Kanji`.
    fn all_parents(&self, k: Kanji) -> HashSet<NodeIndex<u16>> {
        self.entries
            .get(&k)
            .map(|e| {
                e.oya
                    .iter()
                    .filter_map(|o| {
                        let ix = self.index.get(o)?;
                        let mut parents = self.all_parents(*o);
                        parents.insert(*ix);
                        Some(parents)
                    })
                    .flatten()
                    .collect()
            })
            .unwrap_or_else(HashSet::new)
    }
}

/// An entry in the kanji database.
#[derive(Serialize, Deserialize)]
pub struct Entry {
    pub kanji: Kanji,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub oya: Vec<Kanji>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kakushi_oya: Vec<Kanji>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub onyomi: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub daihyou: Vec<String>,
}

/// Open a data file and bring the whole "database" into memory.
pub fn open_db(path: &Path) -> Result<DB, Error> {
    let raw = fs::read_to_string(path).map_err(Error::Io)?;
    let ks: Vec<Entry> = serde_json::from_str(&raw).map_err(Error::Json)?;
    let hm = ks.into_iter().map(|e| (e.kanji, e)).collect();

    Ok(DB::new(hm))
}

/// Write a Kanji "database" into a file by order of its `Kanji`.
pub fn write_db(path: &Path, db: DB) -> Result<(), Error> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .map_err(Error::Io)?;

    let mut entries = db.entries.into_values().collect::<Vec<Entry>>();
    entries.sort_by_key(|e| e.kanji);
    entries.iter_mut().for_each(|e| e.oya.sort());
    serde_json::to_writer_pretty(file, &entries).map_err(Error::Json)
}

/// Apply functions in method-position.
pub trait Apply {
    /// Apply a given function in method-position.
    fn apply<F, U>(self, f: F) -> U
    where
        F: FnOnce(Self) -> U,
        Self: Sized;
}

impl<T> Apply for T {
    fn apply<F, U>(self, f: F) -> U
    where
        F: FnOnce(Self) -> U,
        Self: Sized,
    {
        f(self)
    }
}
