//! JSONL (JSON Lines) parser and serializer for beads format
//!
//! Handles reading and writing beads in JSONL format with versioning support.
//! Each line is a separate JSON object representing one bead.

use crate::graph::Bead;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::Path;

/// JSONL file wrapper for versioned bead data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadEntry {
    /// Format version (for backwards compatibility)
    #[serde(default = "default_version")]
    pub version: u32,

    /// The bead data
    #[serde(flatten)]
    pub bead: Bead,
}

fn default_version() -> u32 {
    1
}

impl BeadEntry {
    /// Create a new entry with the current version
    pub fn new(bead: Bead) -> Self {
        Self {
            version: 1,
            bead,
        }
    }

    /// Create an entry from a bead (current version)
    pub fn from_bead(bead: Bead) -> Self {
        Self::new(bead)
    }
}

/// JSONL reader for beads
pub struct JsonlReader {
    reader: BufReader<File>,
}

impl JsonlReader {
    /// Open a JSONL file for reading
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }

    /// Read all beads from the file
    pub fn read_all(&mut self) -> Result<Vec<Bead>> {
        let mut beads = Vec::new();

        for line_result in self.reader.by_ref().lines() {
            let line = line_result?;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse the JSON line
            let entry: BeadEntry = serde_json::from_str(&line)
                .map_err(|e| crate::AllBeadsError::Parse(format!("Invalid JSONL: {}", e)))?;

            // Handle version compatibility
            match entry.version {
                1 => beads.push(entry.bead),
                _ => {
                    tracing::warn!(version = entry.version, "Unknown bead version, attempting to parse anyway");
                    beads.push(entry.bead);
                }
            }
        }

        Ok(beads)
    }

    /// Read beads one at a time (iterator-style)
    pub fn iter(&mut self) -> JsonlIterator<'_> {
        JsonlIterator { reader: self }
    }
}

/// Iterator over beads in a JSONL file
pub struct JsonlIterator<'a> {
    reader: &'a mut JsonlReader,
}

impl<'a> Iterator for JsonlIterator<'a> {
    type Item = Result<Bead>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();

        loop {
            line.clear();
            match self.reader.reader.read_line(&mut line) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    // Skip empty lines
                    if line.trim().is_empty() {
                        continue;
                    }

                    // Parse the JSON line
                    match serde_json::from_str::<BeadEntry>(&line) {
                        Ok(entry) => return Some(Ok(entry.bead)),
                        Err(e) => return Some(Err(crate::AllBeadsError::Parse(format!("Invalid JSONL: {}", e)))),
                    }
                }
                Err(e) => return Some(Err(e.into())),
            }
        }
    }
}

/// JSONL writer for beads
pub struct JsonlWriter {
    writer: BufWriter<File>,
}

impl JsonlWriter {
    /// Create a new JSONL file for writing
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    /// Append to an existing JSONL file
    pub fn append(path: impl AsRef<Path>) -> Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    /// Write a single bead
    pub fn write(&mut self, bead: &Bead) -> Result<()> {
        let entry = BeadEntry::from_bead(bead.clone());
        let json = serde_json::to_string(&entry)?;
        writeln!(self.writer, "{}", json)?;
        Ok(())
    }

    /// Write multiple beads
    pub fn write_all(&mut self, beads: &[Bead]) -> Result<()> {
        for bead in beads {
            self.write(bead)?;
        }
        Ok(())
    }

    /// Flush the buffer to disk
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

impl Drop for JsonlWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Read all beads from a JSONL file (convenience function)
pub fn read_beads(path: impl AsRef<Path>) -> Result<Vec<Bead>> {
    let mut reader = JsonlReader::open(path)?;
    reader.read_all()
}

/// Write all beads to a JSONL file (convenience function)
pub fn write_beads(path: impl AsRef<Path>, beads: &[Bead]) -> Result<()> {
    let mut writer = JsonlWriter::create(path)?;
    writer.write_all(beads)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_bead(id: &str, title: &str) -> Bead {
        Bead::new(id, title, "test-user")
    }

    #[test]
    fn test_bead_entry_serialization() {
        let bead = create_test_bead("test-1", "Test Bead");
        let entry = BeadEntry::from_bead(bead);

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"id\":\"test-1\""));
    }

    #[test]
    fn test_write_and_read_single_bead() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write a bead
        let bead = create_test_bead("test-1", "Test Bead");
        {
            let mut writer = JsonlWriter::create(path).unwrap();
            writer.write(&bead).unwrap();
            writer.flush().unwrap();
        }

        // Read it back
        let mut reader = JsonlReader::open(path).unwrap();
        let beads = reader.read_all().unwrap();

        assert_eq!(beads.len(), 1);
        assert_eq!(beads[0].id.as_str(), "test-1");
        assert_eq!(beads[0].title, "Test Bead");
    }

    #[test]
    fn test_write_and_read_multiple_beads() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write multiple beads
        let beads = vec![
            create_test_bead("test-1", "First Bead"),
            create_test_bead("test-2", "Second Bead"),
            create_test_bead("test-3", "Third Bead"),
        ];

        write_beads(path, &beads).unwrap();

        // Read them back
        let read_beads = read_beads(path).unwrap();

        assert_eq!(read_beads.len(), 3);
        assert_eq!(read_beads[0].id.as_str(), "test-1");
        assert_eq!(read_beads[1].id.as_str(), "test-2");
        assert_eq!(read_beads[2].id.as_str(), "test-3");
    }

    #[test]
    fn test_iterator() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write beads
        let beads = vec![
            create_test_bead("test-1", "First"),
            create_test_bead("test-2", "Second"),
        ];
        write_beads(path, &beads).unwrap();

        // Read using iterator
        let mut reader = JsonlReader::open(path).unwrap();
        let mut count = 0;
        for result in reader.iter() {
            assert!(result.is_ok());
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_append_mode() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write initial bead
        {
            let mut writer = JsonlWriter::create(path).unwrap();
            writer.write(&create_test_bead("test-1", "First")).unwrap();
        }

        // Append more beads
        {
            let mut writer = JsonlWriter::append(path).unwrap();
            writer.write(&create_test_bead("test-2", "Second")).unwrap();
            writer.write(&create_test_bead("test-3", "Third")).unwrap();
        }

        // Read all
        let beads = read_beads(path).unwrap();
        assert_eq!(beads.len(), 3);
    }

    #[test]
    fn test_empty_lines_skipped() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write with empty lines
        std::fs::write(
            path,
            r#"{"version":1,"id":"test-1","title":"First","status":"open","priority":2,"issue_type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","created_by":"test","dependencies":[],"blocks":[],"labels":[]}

{"version":1,"id":"test-2","title":"Second","status":"open","priority":2,"issue_type":"task","created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","created_by":"test","dependencies":[],"blocks":[],"labels":[]}
"#,
        )
        .unwrap();

        let beads = read_beads(path).unwrap();
        assert_eq!(beads.len(), 2);
    }
}
