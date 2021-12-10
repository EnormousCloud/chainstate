use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Network {
    pub tags: Vec<String>,
    pub endpoint: String,
}

impl Network {
    pub fn new(endpoint: &str, tags: Vec<String>) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            tags: tags.clone(),
        }
    }
}

pub fn from_reader(reader: impl BufRead) -> anyhow::Result<Vec<Network>> {
    let lines: Vec<Network> = reader
        .lines()
        .map(|row| row.unwrap())
        .filter(|row| row.trim().len() > 0)
        .map(|endpoint| Network::new(&endpoint.trim(), vec![]))
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
}
