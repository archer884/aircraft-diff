use std::{
    ffi::{OsStr, OsString},
    fmt::Display,
    fs::File,
    hash::Hash,
    io::{self, BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

use bumpalo::Bump;
use clap::{crate_authors, crate_description, crate_version, Clap};
use hashbrown::HashMap;
use walkdir::WalkDir;

#[derive(Clap, Clone, Debug)]
#[clap(author = crate_authors!(), about = crate_description!(), version = crate_version!())]
struct Opts {
    /// the root of the "left" package tree
    left: String,
    /// the root of the "right" package tree
    right: String,
    /// file containing keys to ignore
    #[clap(short, long)]
    ignore: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key<'a> {
    section: &'a str,
    property: String,
}

impl Display for Key<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.section, self.property)
    }
}

struct Difference<'a> {
    key: Key<'a>,
    left: String,
    right: String,
}

fn main() -> io::Result<()> {
    let opts = Opts::parse();

    let tree = read_common_tree(&opts.left, &opts.right);
    let store = Bump::new();

    for (file, (left, right)) in tree {
        let differences: Vec<_> = diff_paths(&left, &right, &store)?.collect();

        if !differences.is_empty() {
            println!("# {} ({})", file.to_string_lossy(), differences.len());
            for difference in differences {
                println!(
                    "  {}\n    {}\n    {}",
                    difference.key, difference.left, difference.right
                );
            }
        }
    }

    Ok(())
}

fn read_common_tree(
    left: &str,
    right: &str,
) -> impl Iterator<Item = (OsString, (PathBuf, PathBuf))> {
    let left: HashMap<_, _> = read_tree(left)
        .map(|x| (x.file_name().unwrap().to_owned(), x))
        .collect();
    let mut right: HashMap<_, _> = read_tree(right)
        .map(|x| (x.file_name().unwrap().to_owned(), x))
        .collect();

    left.into_iter()
        .filter_map(move |(file, left)| right.remove(&file).map(|right| (file, (left, right))))
}

fn read_tree(root: &str) -> impl Iterator<Item = PathBuf> {
    let tgt_ext = OsStr::new("cfg");
    let tgt_ext_cap = OsStr::new("CFG");

    WalkDir::new(root).into_iter().filter_map(move |entry| {
        entry
            .ok()
            .filter(|x| {
                x.path()
                    .extension()
                    .map(|ext| ext == tgt_ext || ext == tgt_ext_cap)
                    .unwrap_or_default()
            })
            .map(|x| x.into_path())
    })
}

fn diff_paths(
    left: impl AsRef<Path>,
    right: impl AsRef<Path>,
    store: &Bump,
) -> io::Result<impl Iterator<Item = Difference>> {
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
