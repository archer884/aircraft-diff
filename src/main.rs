use std::{
    fmt::Display,
    fs::File,
    hash::Hash,
    io::{self, BufRead, BufReader, Read},
};

use hashbrown::HashMap;
use slotmap::{DefaultKey, SlotMap};

type SectionStore = SlotMap<DefaultKey, String>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    section: DefaultKey,
    property: String,
}

struct Difference {
    key: Key,
    left: String,
    right: String,
}

impl Difference {
    fn format_key<'a>(&'a self, store: &'a SectionStore) -> KeyFormat {
        KeyFormat {
            property: &self.key.property,
            key: self.key.section,
            store,
        }
    }
}

struct KeyFormat<'a> {
    property: &'a str,
    key: DefaultKey,
    store: &'a SectionStore,
}

impl Display for KeyFormat<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.store[self.key], self.property)
    }
}

struct Comparer<'a> {
    store: &'a mut SectionStore,
}

impl<'a> Comparer<'a> {
    fn with_store(store: &'a mut SectionStore) -> Self {
        Self { store }
    }

    fn diff_paths(
        &mut self,
        left: &str,
        right: &str,
    ) -> io::Result<impl Iterator<Item = Difference>> {
        let left = File::open(left)?;
        let right = File::open(right)?;
        Ok(self.diff(left, right))
    }

    fn diff(&mut self, left: impl Read, right: impl Read) -> impl Iterator<Item = Difference> {
        let left = self.read_to_map(left);
        let mut right = self.read_to_map(right);

        left.into_iter().filter_map(move |(key, value)| {
            let other = right.remove(&key)?;
            if value != other {
                Some(Difference {
                    key,
                    left: value,
                    right: other,
                })
            } else {
                None
            }
        })
    }

    fn read_to_map(&mut self, config: impl Read) -> HashMap<Key, String> {
        let mut section = self.store.insert(String::from("root"));
        let mut map = HashMap::new();

        let config = BufReader::new(config);
        let config = config
            .lines()
            .filter_map(Result::ok)
            .filter(|x| !x.is_empty() && !is_whitespace(&x));

        for line in config {
            let line = match line.find(';') {
                Some(idx) => {
                    let (line, _comment) = line.split_at(idx);
                    line.trim()
                }
                None => line.trim(),
            };

            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let section_name = String::from(&line[1..(line.len() - 1)]);
                section = self.store.insert(section_name);
                continue;
            }

            if let Some(idx) = line.find('=') {
                let (key, value) = line.split_at(idx);
                map.insert(
                    Key {
                        section,
                        property: key.trim().to_string(),
                    },
                    value.trim().to_string(),
                );
            }
        }

        map
    }
}

fn main() -> io::Result<()> {
    let mut store = SlotMap::new();
    let mut comparer = Comparer::with_store(&mut store);

    let left = "../resource/engines.old.cfg";
    let right = "../resource/engines.new.cfg";

    let differences = comparer.diff_paths(left, right)?;

    for diff in differences {
        println!(
            "{}: {} / {}",
            diff.format_key(&store),
            diff.left,
            diff.right
        );
    }

    Ok(())
}

fn is_whitespace(s: &str) -> bool {
    s.chars().all(|x| x.is_whitespace())
}
