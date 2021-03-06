use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::iter::FromIterator;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Network {
    pub tags: HashSet<String>,
    pub endpoint: String,
}

#[derive(Debug, Clone)]
pub enum TagMatcher {
    Has(String),
    DoesntHave(String),
}

impl TagMatcher {
    pub fn from(src: &str) -> Option<Self> {
        if src.len() == 0 {
            return None;
        }
        if src.len() > 1 {
            let first_ch: String = src.chars().take(1).collect();
            if first_ch == "-" {
                let res = src.chars().skip(1).collect();
                return Some(Self::DoesntHave(res));
            }
        }
        Some(Self::Has(src.to_string()))
    }
}

impl Network {
    pub fn new(endpoint: &str, tags: HashSet<String>) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            tags: tags.clone(),
        }
    }
    pub fn has_all(&self, tags: &HashSet<String>) -> bool {
        if tags.len() > 0 {
            for t in tags {
                if let Some(tm) = TagMatcher::from(&t) {
                    if match tm {
                        TagMatcher::Has(x) => !self.tags.contains(&x),
                        TagMatcher::DoesntHave(x) => self.tags.contains(&x),
                    } {
                        return false;
                    }
                }
            }
        }
        return true;
    }
}

pub fn from_reader(reader: impl BufRead) -> anyhow::Result<Vec<Network>> {
    let mut tags: HashSet<String> = HashSet::new();
    let lines: Vec<Network> = reader
        .lines()
        .map(|row| row.unwrap())
        .filter(|row| row.trim().len() > 0)
        .map(|row| {
            let start = row.trim().chars().take(1).collect::<String>();
            if start == "#" {
                let remainder = row.trim().chars().skip(1).collect::<String>();
                let iter = remainder.split(",").map(|x| x.trim().to_string());
                tags = HashSet::from_iter(iter);
                None
            } else {
                let t = tags.clone();
                tags = HashSet::new();
                Some(Network::new(&row.trim(), t))
            }
        })
        .flatten()
        .collect();
    Ok(lines)
}

pub fn from_file(source: &str) -> anyhow::Result<Vec<Network>> {
    let file = File::open(Path::new(source)).unwrap();
    return from_reader(io::BufReader::new(file));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn it_reads_plain() {
        let input = r#"
        http://test.com
        https://test2.com
        "#;
        let cursor = io::Cursor::new(input.as_bytes());
        let output = from_reader(cursor).unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].endpoint, "http://test.com");
        assert_eq!(output[1].endpoint, "https://test2.com");
    }

    #[test]
    pub fn it_reads_with_tags() {
        let input = r#"
        #one, and, test
        http://test.com
        https://test2.com
        #third, no test
        https://test3.com
        "#;
        let cursor = io::Cursor::new(input.as_bytes());
        let output = from_reader(cursor).unwrap();
        assert_eq!(output.len(), 3);
        assert_eq!(output[0].endpoint, "http://test.com");
        let mut tags1 = HashSet::new();
        tags1.insert("one".to_string());
        tags1.insert("and".to_string());
        tags1.insert("test".to_string());
        assert_eq!(output[0].tags, tags1);
        assert_eq!(output[1].endpoint, "https://test2.com");
        assert_eq!(output[1].tags, HashSet::new());

        assert_eq!(output[2].endpoint, "https://test3.com");
        let mut tags3 = HashSet::new();
        tags3.insert("third".to_string());
        tags3.insert("no test".to_string());
        assert_eq!(output[2].tags, tags3);
    }

    #[test]
    pub fn it_matches_tags() {
        let mut tags = HashSet::new();
        tags.insert("one".to_string());
        tags.insert("two".to_string());
        let n = Network::new("test", tags.clone());
        assert_eq!(n.has_all(&tags), true);

        let mut t2 = tags.clone();
        t2.remove("two");
        let n2 = Network::new("test", t2.clone());
        assert_eq!(n2.has_all(&tags), false);
    }

    #[test]
    pub fn it_matches_tags_with_exclusion() {
        let mut tags = HashSet::new();
        tags.insert("one".to_string());
        tags.insert("two".to_string());
        let n = Network::new("test", tags.clone());

        let mut t = HashSet::new();
        t.insert("one".to_string());
        t.insert("-three".to_string());
        assert_eq!(n.has_all(&t), true);
    }
}
