//! SQLite cache implementation

use crate::graph::{Bead, BeadId, FederatedGraph, Priority, Rig, Status};
use crate::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Path to SQLite database file
    pub path: PathBuf,

    /// Cache expiration duration
    pub ttl: Duration,

    /// Enable WAL mode for better concurrency
    pub wal_mode: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        // Always use ~/.config for consistency across platforms (macOS, Linux)
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".config");
        path.push("allbeads");
        path.push("cache.db");

        Self {
            path,
            ttl: Duration::from_secs(300), // 5 minutes
            wal_mode: true,
        }
    }
}

/// SQLite cache for FederatedGraph
pub struct Cache {
    conn: Connection,
    config: CacheConfig,
}

impl Cache {
    /// Open or create a cache database
    pub fn new(config: CacheConfig) -> Result<Self> {
        // Create parent directory if needed
        if let Some(parent) = config.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        tracing::info!(path = %config.path.display(), "Opening cache database");

        let conn = Connection::open(&config.path)?;

        // Enable WAL mode for better concurrency
        if config.wal_mode {
            conn.pragma_update(None, "journal_mode", &"WAL")?;
        }

        // Create cache instance
        let cache = Self { conn, config };

        // Initialize schema
        cache.init_schema()?;

        Ok(cache)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS cache_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS beads (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                priority INTEGER NOT NULL,
                issue_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                created_by TEXT NOT NULL,
                assignee TEXT,
                labels TEXT NOT NULL,
                notes TEXT,
                context TEXT NOT NULL,
                cached_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS dependencies (
                bead_id TEXT NOT NULL,
                depends_on TEXT NOT NULL,
                PRIMARY KEY (bead_id, depends_on),
                FOREIGN KEY (bead_id) REFERENCES beads(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS blocks (
                bead_id TEXT NOT NULL,
                blocks_id TEXT NOT NULL,
                PRIMARY KEY (bead_id, blocks_id),
                FOREIGN KEY (bead_id) REFERENCES beads(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS rigs (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                remote TEXT NOT NULL,
                auth_strategy TEXT NOT NULL,
                prefix TEXT NOT NULL,
                context TEXT NOT NULL,
                cached_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_beads_status ON beads(status);
            CREATE INDEX IF NOT EXISTS idx_beads_context ON beads(context);
            CREATE INDEX IF NOT EXISTS idx_beads_priority ON beads(priority);
            CREATE INDEX IF NOT EXISTS idx_dependencies_bead ON dependencies(bead_id);
            CREATE INDEX IF NOT EXISTS idx_blocks_bead ON blocks(bead_id);
            "#,
        )?;

        Ok(())
    }

    /// Store a FederatedGraph in the cache
    pub fn store_graph(&self, graph: &FederatedGraph) -> Result<()> {
        tracing::debug!(beads = graph.stats().total_beads, "Storing graph in cache");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Start transaction
        let tx = self.conn.unchecked_transaction()?;

        // Clear existing data
        tx.execute("DELETE FROM dependencies", [])?;
        tx.execute("DELETE FROM blocks", [])?;
        tx.execute("DELETE FROM beads", [])?;
        tx.execute("DELETE FROM rigs", [])?;

        // Store all beads
        for bead in graph.beads.values() {
            self.store_bead_tx(&tx, bead, now)?;
        }

        // Store all rigs
        for rig in graph.rigs.values() {
            self.store_rig_tx(&tx, rig, now)?;
        }

        // Update metadata
        tx.execute(
            "INSERT OR REPLACE INTO cache_metadata (key, value, updated_at) VALUES (?, ?, ?)",
            params!["last_update", now.to_string(), now],
        )?;

        tx.commit()?;

        tracing::info!("Graph stored in cache successfully");
        Ok(())
    }

    /// Store a single bead within a transaction
    fn store_bead_tx(&self, tx: &Connection, bead: &Bead, timestamp: i64) -> Result<()> {
        // Extract context from labels (tags starting with @)
        let context = bead
            .labels
            .iter()
            .find(|l| l.starts_with('@'))
            .map(|l| l.trim_start_matches('@'))
            .unwrap_or("unknown");

        // Serialize labels as comma-separated
        let labels_str = bead.labels.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(",");

        // Insert bead
        tx.execute(
            r#"
            INSERT INTO beads (
                id, title, description, status, priority, issue_type,
                created_at, updated_at, created_by, assignee,
                labels, notes, context, cached_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                bead.id.as_str(),
                &bead.title,
                bead.description.as_deref(),
                status_to_str(bead.status),
                priority_to_int(bead.priority),
                issue_type_to_str(bead.issue_type),
                &bead.created_at,
                &bead.updated_at,
                &bead.created_by,
                bead.assignee.as_deref(),
                labels_str,
                bead.notes.as_deref(),
                context,
                timestamp,
            ],
        )?;

        // Store dependencies
        for dep_id in &bead.dependencies {
            tx.execute(
                "INSERT INTO dependencies (bead_id, depends_on) VALUES (?, ?)",
                params![bead.id.as_str(), dep_id.as_str()],
            )?;
        }

        // Store blocks relationships
        for blocks_id in &bead.blocks {
            tx.execute(
                "INSERT INTO blocks (bead_id, blocks_id) VALUES (?, ?)",
                params![bead.id.as_str(), blocks_id.as_str()],
            )?;
        }

        Ok(())
    }

    /// Store a single rig within a transaction
    fn store_rig_tx(&self, tx: &Connection, rig: &Rig, timestamp: i64) -> Result<()> {
        tx.execute(
            r#"
            INSERT INTO rigs (id, path, remote, auth_strategy, prefix, context, cached_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                rig.id.as_str(),
                rig.path.to_string_lossy(),
                &rig.remote,
                rig_auth_to_str(&rig.auth_strategy),
                &rig.prefix,
                &rig.context,
                timestamp,
            ],
        )?;

        Ok(())
    }

    /// Load a FederatedGraph from the cache
    pub fn load_graph(&self) -> Result<Option<FederatedGraph>> {
        // Check if cache is expired
        if self.is_expired()? {
            tracing::debug!("Cache is expired");
            return Ok(None);
        }

        tracing::debug!("Loading graph from cache");

        let mut graph = FederatedGraph::new();

        // Load all beads
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, title, description, status, priority, issue_type,
                   created_at, updated_at, created_by, assignee,
                   labels, notes
            FROM beads
            "#,
        )?;

        let beads = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let labels_str: String = row.get(10)?;
            let labels: std::collections::HashSet<String> = labels_str
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            Ok(Bead {
                id: BeadId::new(id),
                title: row.get(1)?,
                description: row.get(2)?,
                status: str_to_status(row.get::<_, String>(3)?.as_str()),
                priority: int_to_priority(row.get(4)?),
                issue_type: str_to_issue_type(row.get::<_, String>(5)?.as_str()),
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                created_by: row.get(8)?,
                assignee: row.get(9)?,
                labels,
                notes: row.get(11)?,
                dependencies: Vec::new(), // Will load separately
                blocks: Vec::new(),       // Will load separately
            })
        })?;

        for bead_result in beads {
            let mut bead = bead_result?;

            // Load dependencies
            let mut dep_stmt = self.conn.prepare(
                "SELECT depends_on FROM dependencies WHERE bead_id = ?",
            )?;
            let deps = dep_stmt.query_map([bead.id.as_str()], |row| row.get::<_, String>(0))?;
            for dep in deps {
                bead.dependencies.push(BeadId::new(dep?));
            }

            // Load blocks
            let mut blocks_stmt = self.conn.prepare(
                "SELECT blocks_id FROM blocks WHERE bead_id = ?",
            )?;
            let blocks = blocks_stmt.query_map([bead.id.as_str()], |row| row.get::<_, String>(0))?;
            for block_id in blocks {
                bead.blocks.push(BeadId::new(block_id?));
            }

            graph.add_bead(bead);
        }

        tracing::info!(beads = graph.stats().total_beads, "Graph loaded from cache");
        Ok(Some(graph))
    }

    /// Check if the cache is expired
    pub fn is_expired(&self) -> Result<bool> {
        let last_update: Option<i64> = self
            .conn
            .query_row(
                "SELECT updated_at FROM cache_metadata WHERE key = 'last_update'",
                [],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(last_update) = last_update {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;

            let age = Duration::from_millis((now - last_update) as u64);
            Ok(age > self.config.ttl)
        } else {
            // No metadata means cache is empty/expired
            Ok(true)
        }
    }

    /// Clear all cached data
    pub fn clear(&self) -> Result<()> {
        tracing::info!("Clearing cache");

        self.conn.execute("DELETE FROM dependencies", [])?;
        self.conn.execute("DELETE FROM blocks", [])?;
        self.conn.execute("DELETE FROM beads", [])?;
        self.conn.execute("DELETE FROM rigs", [])?;
        self.conn.execute("DELETE FROM cache_metadata", [])?;

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let bead_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM beads", [], |row| row.get(0))?;
        let rig_count: i64 = self.conn.query_row("SELECT COUNT(*) FROM rigs", [], |row| row.get(0))?;

        let last_update: Option<i64> = self
            .conn
            .query_row(
                "SELECT updated_at FROM cache_metadata WHERE key = 'last_update'",
                [],
                |row| row.get(0),
            )
            .optional()?;

        let age = if let Some(last_update) = last_update {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            Some(Duration::from_millis((now - last_update) as u64))
        } else {
            None
        };

        Ok(CacheStats {
            bead_count: bead_count as usize,
            rig_count: rig_count as usize,
            last_update,
            age,
            is_expired: self.is_expired()?,
        })
    }

    /// Get the database path
    pub fn path(&self) -> &Path {
        &self.config.path
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub bead_count: usize,
    pub rig_count: usize,
    pub last_update: Option<i64>,
    pub age: Option<Duration>,
    pub is_expired: bool,
}

// Helper functions for type conversions

fn status_to_str(status: Status) -> &'static str {
    match status {
        Status::Open => "open",
        Status::InProgress => "in_progress",
        Status::Blocked => "blocked",
        Status::Deferred => "deferred",
        Status::Closed => "closed",
        Status::Tombstone => "tombstone",
    }
}

fn str_to_status(s: &str) -> Status {
    match s {
        "in_progress" => Status::InProgress,
        "blocked" => Status::Blocked,
        "deferred" => Status::Deferred,
        "closed" => Status::Closed,
        "tombstone" => Status::Tombstone,
        _ => Status::Open,
    }
}

fn priority_to_int(priority: Priority) -> i32 {
    priority as i32
}

fn int_to_priority(i: i32) -> Priority {
    match i {
        0 => Priority::P0,
        1 => Priority::P1,
        2 => Priority::P2,
        3 => Priority::P3,
        _ => Priority::P4,
    }
}

fn issue_type_to_str(issue_type: crate::graph::IssueType) -> &'static str {
    match issue_type {
        crate::graph::IssueType::Bug => "bug",
        crate::graph::IssueType::Feature => "feature",
        crate::graph::IssueType::Task => "task",
        crate::graph::IssueType::Epic => "epic",
        crate::graph::IssueType::Chore => "chore",
        crate::graph::IssueType::MergeRequest => "merge_request",
        crate::graph::IssueType::Molecule => "molecule",
        crate::graph::IssueType::Gate => "gate",
    }
}

fn str_to_issue_type(s: &str) -> crate::graph::IssueType {
    match s {
        "bug" => crate::graph::IssueType::Bug,
        "feature" => crate::graph::IssueType::Feature,
        "epic" => crate::graph::IssueType::Epic,
        "chore" => crate::graph::IssueType::Chore,
        "merge_request" => crate::graph::IssueType::MergeRequest,
        "molecule" => crate::graph::IssueType::Molecule,
        "gate" => crate::graph::IssueType::Gate,
        _ => crate::graph::IssueType::Task,
    }
}

fn rig_auth_to_str(auth: &crate::graph::RigAuthStrategy) -> &'static str {
    match auth {
        crate::graph::RigAuthStrategy::SshAgent => "ssh_agent",
        crate::graph::RigAuthStrategy::GhEnterpriseToken => "gh_enterprise_token",
        crate::graph::RigAuthStrategy::PersonalAccessToken => "personal_access_token",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Bead;
    use tempfile::NamedTempFile;

    #[test]
    fn test_cache_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = CacheConfig {
            path: temp_file.path().to_path_buf(),
            ..Default::default()
        };

        let cache = Cache::new(config).unwrap();
        assert!(cache.path().exists());
    }

    #[test]
    fn test_cache_store_and_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = CacheConfig {
            path: temp_file.path().to_path_buf(),
            ttl: Duration::from_secs(3600), // 1 hour
            ..Default::default()
        };

        let cache = Cache::new(config).unwrap();

        // Create a graph with some beads
        let mut graph = FederatedGraph::new();
        let mut bead1 = Bead::new("ab-123", "Test Issue 1", "alice");
        bead1.add_label("@work");
        let mut bead2 = Bead::new("ab-456", "Test Issue 2", "bob");
        bead2.add_label("@personal");

        graph.add_bead(bead1);
        graph.add_bead(bead2);

        // Store the graph
        cache.store_graph(&graph).unwrap();

        // Load it back
        let loaded = cache.load_graph().unwrap();
        assert!(loaded.is_some());

        let loaded_graph = loaded.unwrap();
        assert_eq!(loaded_graph.stats().total_beads, 2);
    }

    #[test]
    fn test_cache_expiration() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = CacheConfig {
            path: temp_file.path().to_path_buf(),
            ttl: Duration::from_millis(100), // 100ms
            ..Default::default()
        };

        let cache = Cache::new(config).unwrap();

        // Initially expired (no data)
        assert!(cache.is_expired().unwrap());

        // Store some data
        let graph = FederatedGraph::new();
        cache.store_graph(&graph).unwrap();

        // Should not be expired immediately
        assert!(!cache.is_expired().unwrap());

        // Wait for expiration (add extra time for reliability)
        std::thread::sleep(Duration::from_millis(200));
        assert!(cache.is_expired().unwrap());
    }

    #[test]
    fn test_cache_clear() {
        let temp_file = NamedTempFile::new().unwrap();
        let config = CacheConfig {
            path: temp_file.path().to_path_buf(),
            ..Default::default()
        };

        let cache = Cache::new(config).unwrap();

        // Store some data
        let mut graph = FederatedGraph::new();
        graph.add_bead(Bead::new("ab-123", "Test", "alice"));
        cache.store_graph(&graph).unwrap();

        // Verify it's there
        let stats = cache.stats().unwrap();
        assert_eq!(stats.bead_count, 1);

        // Clear
        cache.clear().unwrap();

        // Verify it's gone
        let stats = cache.stats().unwrap();
        assert_eq!(stats.bead_count, 0);
    }
}
