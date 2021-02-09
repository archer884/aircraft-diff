use std::{
    fmt::Display,
    fs::File,
    hash::Hash,
    io::{self, BufRead, BufReader, Read},
};

use bumpalo::Bump;
use hashbrown::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key<'a> {
    section: &'a str,
    property: String,
}

struct Difference<'a> {
    key: Key<'a>,
    left: String,
    right: String,
}

impl Display for Difference<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}\n  {}\n  {}",
            self.key.section, self.key.property, self.left, self.right
        )
    }
}

fn main() -> io::Result<()> {
    let mut store = Bump::new();

    let left = "resource/engines.old.cfg";
    let right = "resource/engines.new.cfg";

    let differences = diff_paths(left, right, &mut store)?;

    for diff in differences {
        println!("{}", diff);
    }

    Ok(())
}

fn diff_paths<'a>(
    left: &str,
    right: &str,
    store: &'a mut Bump,
) -> io::Result<impl Iterator<Item = Difference<'a>>> {
    let left = File::open(left)?;
    let right = File::open(right)?;
    Ok(diff(left, right, store))
}

fn diff(left: impl Read, right: impl Read, store: &Bump) -> impl Iterator<Item = Difference> + '_ {
    let left = read_to_map(left, store);
    let mut right = read_to_map(right, store);

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

fn read_to_map(config: impl Read, store: &Bump) -> HashMap<Key, String> {
    let mut section = store.alloc_str("root");
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
            section = store.alloc_str(&line[1..(line.len() - 1)]);
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

fn is_whitespace(s: &str) -> bool {
    s.chars().all(|x| x.is_whitespace())
}
