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
                if !self.tags.contains(t) {
                    return false;
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
}
