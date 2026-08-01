use alloc::collections::BTreeMap;
use std::env::var;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::PathBuf;

pub struct Tests(BTreeMap<String, String>);

impl Tests {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    pub fn load() -> Self {
        let mut this = Self(BTreeMap::new());
        let Ok(raw) = fs::read_to_string(Self::path()) else {
            return this;
        };
        let mut lines = raw.lines();
        while let Some(header) = lines.next()
            && !header.is_empty()
        {
            let (key, nb_lines) = header.split_once(' ').unwrap();
            let nb = nb_lines.parse().unwrap();
            let output = lines.by_ref().take(nb).collect::<Vec<_>>();
            assert_eq!(output.len(), nb, "Missing lines");
            assert_eq!(this.0.insert(key.to_owned(), output.join("\n")), None, "Duplicate key");
        }
        this
    }

    fn path() -> PathBuf {
        PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests")
            .join("output")
    }

    pub fn remove(&mut self, key: &str) {
        self.0.remove(key);
    }

    pub fn set(&mut self, key: String, content: &str) {
        self.0
            .entry(key)
            .and_modify(|old| content.clone_into(old))
            .or_insert_with(|| content.to_owned());
    }

    pub fn store(&self) {
        let mut fd = File::create(Self::path()).unwrap();
        for (key, val) in &self.0 {
            let lines = val
                .chars()
                .filter(|ch| *ch == '\n')
                .count()
                .saturating_add(1);
            assert!(key.find(' ').is_none(), "breaks parsing logic");
            writeln!(fd, "{key} {lines}\n{val}").unwrap();
        }
    }
}
