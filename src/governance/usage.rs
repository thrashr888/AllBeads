//! Agent Usage Tracking
//!
//! Stores and retrieves agent detection history over time.
//! Used to track adoption metrics and trends.

use crate::governance::agents::{AgentScanResult, AgentType};
use crate::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Agent usage record stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub context_name: String,
    pub repo_path: String,
    pub agents: Vec<AgentType>,
    pub agent_count: usize,
}

/// Usage statistics for a time period
#[derive(Debug, Clone, Default, Serialize)]
pub struct UsageStats {
    pub total_scans: usize,
    pub repos_with_agents: usize,
    pub repos_without_agents: usize,
    pub agent_counts: HashMap<String, usize>,
    pub adoption_rate: f64,
}

/// Usage trend data
#[derive(Debug, Clone, Serialize)]
pub struct UsageTrend {
    pub date: String,
    pub repos_scanned: usize,
    pub repos_with_agents: usize,
    pub adoption_rate: f64,
}

/// Usage tracking storage
pub struct UsageStorage {
    conn: Connection,
}

impl UsageStorage {
    /// Create a new usage storage at the given path
    pub fn new(db_path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        let storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Open usage storage in the default location
    pub fn open_default() -> Result<Self> {
        let path = Self::default_path();
        Self::new(&path)
    }

    /// Get the default database path
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("allbeads")
            .join("governance")
            .join("usage.db")
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS usage_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                context_name TEXT NOT NULL,
                repo_path TEXT NOT NULL,
                agents TEXT NOT NULL,
                agent_count INTEGER NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_records(timestamp)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_usage_context ON usage_records(context_name)",
            [],
        )?;

        Ok(())
    }

    /// Record an agent scan result
    pub fn record_scan(
        &self,
        context_name: &str,
        repo_path: &str,
        scan_result: &AgentScanResult,
    ) -> Result<()> {
        let agents: Vec<String> = scan_result
            .agent_types()
            .iter()
            .map(|a| a.id().to_string())
            .collect();
        let agents_json = serde_json::to_string(&agents)?;
        let timestamp = Utc::now().to_rfc3339();
        let agent_count = agents.len();

        self.conn.execute(
            "INSERT INTO usage_records (timestamp, context_name, repo_path, agents, agent_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![timestamp, context_name, repo_path, agents_json, agent_count],
        )?;

        Ok(())
    }

    /// Get usage stats for a time period
    pub fn get_stats(&self, days: u32) -> Result<UsageStats> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        // Get unique repos scanned (most recent scan per repo)
        let mut stmt = self.conn.prepare(
            "SELECT context_name, repo_path, agents, agent_count
             FROM usage_records
             WHERE timestamp > ?1
             GROUP BY context_name, repo_path
             HAVING timestamp = MAX(timestamp)",
        )?;

        let mut stats = UsageStats::default();
        let mut rows = stmt.query(params![cutoff_str])?;

        while let Some(row) = rows.next()? {
            stats.total_scans += 1;
            let agent_count: i64 = row.get(3)?;
            if agent_count > 0 {
                stats.repos_with_agents += 1;
                let agents_json: String = row.get(2)?;
                let agents: Vec<String> = serde_json::from_str(&agents_json)?;
                for agent in agents {
                    *stats.agent_counts.entry(agent).or_insert(0) += 1;
                }
            } else {
                stats.repos_without_agents += 1;
            }
        }

        if stats.total_scans > 0 {
            stats.adoption_rate =
                (stats.repos_with_agents as f64 / stats.total_scans as f64) * 100.0;
        }

        Ok(stats)
    }

    /// Get usage trends over time (daily aggregation)
    pub fn get_trends(&self, days: u32) -> Result<Vec<UsageTrend>> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let mut stmt = self.conn.prepare(
            "SELECT date(timestamp) as day,
                    COUNT(DISTINCT repo_path) as repos_scanned,
                    SUM(CASE WHEN agent_count > 0 THEN 1 ELSE 0 END) as repos_with_agents
             FROM usage_records
             WHERE timestamp > ?1
             GROUP BY day
             ORDER BY day",
        )?;

        let mut trends = Vec::new();
        let mut rows = stmt.query(params![cutoff_str])?;

        while let Some(row) = rows.next()? {
            let date: String = row.get(0)?;
            let repos_scanned: i64 = row.get(1)?;
            let repos_with_agents: i64 = row.get(2)?;

            let adoption_rate = if repos_scanned > 0 {
                (repos_with_agents as f64 / repos_scanned as f64) * 100.0
            } else {
                0.0
            };

            trends.push(UsageTrend {
                date,
                repos_scanned: repos_scanned as usize,
                repos_with_agents: repos_with_agents as usize,
                adoption_rate,
            });
        }

        Ok(trends)
    }

    /// Get the last scan timestamp for a repo
    pub fn get_last_scan(&self, context_name: &str, repo_path: &str) -> Result<Option<DateTime<Utc>>> {
        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT timestamp FROM usage_records
                 WHERE context_name = ?1 AND repo_path = ?2
                 ORDER BY timestamp DESC LIMIT 1",
                params![context_name, repo_path],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(ts) = result {
            let dt = DateTime::parse_from_rfc3339(&ts)
                .map(|d| d.with_timezone(&Utc))
                .ok();
            Ok(dt)
        } else {
            Ok(None)
        }
    }

    /// Get total record count
    pub fn get_record_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM usage_records",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Clean up old records (keep last N days)
    pub fn cleanup(&self, keep_days: u32) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(keep_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = self.conn.execute(
            "DELETE FROM usage_records WHERE timestamp < ?1",
            params![cutoff_str],
        )?;

        Ok(deleted)
    }
}

/// Print usage statistics
pub fn print_usage_stats(stats: &UsageStats, trends: &[UsageTrend], days: u32) {
    println!("Agent Usage Statistics (last {} days)", days);
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    println!("Summary:");
    println!("  Total repositories scanned: {}", stats.total_scans);
    println!("  Repositories with agents:   {}", stats.repos_with_agents);
    println!("  Repositories without:       {}", stats.repos_without_agents);
    println!("  Adoption rate:              {:.1}%", stats.adoption_rate);
    println!();

    if !stats.agent_counts.is_empty() {
        println!("Agent Distribution:");
        let mut sorted: Vec<_> = stats.agent_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (agent, count) in sorted {
            println!("  {}: {} repos", agent, count);
        }
        println!();
    }

    if !trends.is_empty() {
        println!("Adoption Trend:");
        for trend in trends.iter().rev().take(7) {
            let bar_len = (trend.adoption_rate / 5.0) as usize;
            let bar = "█".repeat(bar_len);
            println!(
                "  {} {} {:.1}% ({}/{})",
                trend.date, bar, trend.adoption_rate, trend.repos_with_agents, trend.repos_scanned
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governance::agents::{AgentDetection, DetectionConfidence};
    use tempfile::TempDir;

    #[test]
    fn test_usage_storage_new() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = UsageStorage::new(&db_path).unwrap();
        assert_eq!(storage.get_record_count().unwrap(), 0);
    }

    #[test]
    fn test_record_and_stats() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let storage = UsageStorage::new(&db_path).unwrap();

        // Create a mock scan result
        let scan = AgentScanResult {
            scanned_path: dir.path().to_path_buf(),
            detections: vec![AgentDetection {
                agent: AgentType::Claude,
                confidence: DetectionConfidence::High,
                config_path: Some(dir.path().join("CLAUDE.md")),
                evidence: vec!["CLAUDE.md found".to_string()],
            }],
        };

        storage.record_scan("test", "/test/path", &scan).unwrap();
        assert_eq!(storage.get_record_count().unwrap(), 1);

        let stats = storage.get_stats(7).unwrap();
        assert_eq!(stats.total_scans, 1);
        assert_eq!(stats.repos_with_agents, 1);
    }
}
