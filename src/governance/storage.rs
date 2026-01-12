//! Storage for policy check results

use super::policy::{Policy, PolicySeverity, PolicyType};
use super::rules::CheckResult;
use rusqlite::{Connection, Result as SqlResult};
use std::path::Path;

/// Storage for policy configurations and check results
pub struct PolicyStorage {
    conn: Connection,
}

impl PolicyStorage {
    /// Create a new storage instance with an in-memory database
    pub fn in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Create a new storage instance with a file-backed database
    pub fn new(path: impl AsRef<Path>) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let storage = Self { conn };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS policies (
                name TEXT PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 1,
                description TEXT,
                policy_type TEXT NOT NULL,
                severity TEXT NOT NULL DEFAULT 'warning',
                config TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS check_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                policy_name TEXT NOT NULL,
                passed INTEGER NOT NULL,
                message TEXT,
                affected_beads TEXT,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (policy_name) REFERENCES policies(name)
            );

            CREATE INDEX IF NOT EXISTS idx_check_results_policy ON check_results(policy_name);
            CREATE INDEX IF NOT EXISTS idx_check_results_timestamp ON check_results(timestamp);
            "#,
        )?;
        Ok(())
    }

    /// Save a policy
    pub fn save_policy(&self, policy: &Policy) -> SqlResult<()> {
        let policy_type = serde_json::to_string(&policy.policy_type).unwrap_or_default();
        let config = serde_json::to_string(&policy.config).unwrap_or_default();
        let severity = match policy.severity {
            PolicySeverity::Error => "error",
            PolicySeverity::Warning => "warning",
            PolicySeverity::Info => "info",
        };

        self.conn.execute(
            r#"
            INSERT OR REPLACE INTO policies (name, enabled, description, policy_type, severity, config, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
            "#,
            (
                &policy.name,
                policy.enabled as i32,
                &policy.description,
                &policy_type,
                severity,
                &config,
            ),
        )?;
        Ok(())
    }

    /// Load all policies
    pub fn load_policies(&self) -> SqlResult<Vec<Policy>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, enabled, description, policy_type, severity, config FROM policies",
        )?;

        let policies = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let enabled: i32 = row.get(1)?;
                let description: String = row.get(2)?;
                let policy_type_str: String = row.get(3)?;
                let severity_str: String = row.get(4)?;
                let config_str: String = row.get(5)?;

                let policy_type: PolicyType =
                    serde_json::from_str(&policy_type_str).unwrap_or(PolicyType::RequireDescription);
                let config = serde_json::from_str(&config_str).unwrap_or_default();
                let severity = match severity_str.as_str() {
                    "error" => PolicySeverity::Error,
                    "info" => PolicySeverity::Info,
                    _ => PolicySeverity::Warning,
                };

                Ok(Policy {
                    name,
                    enabled: enabled != 0,
                    description,
                    policy_type,
                    config,
                    severity,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(policies)
    }

    /// Save a check result
    pub fn save_result(&self, result: &CheckResult) -> SqlResult<()> {
        let affected_beads = result.affected_beads.join(",");
        self.conn.execute(
            r#"
            INSERT INTO check_results (policy_name, passed, message, affected_beads, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            (
                &result.policy_name,
                result.passed as i32,
                &result.message,
                &affected_beads,
                &result.timestamp,
            ),
        )?;
        Ok(())
    }

    /// Get recent check results
    pub fn recent_results(&self, limit: usize) -> SqlResult<Vec<CheckResult>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT policy_name, passed, message, affected_beads, timestamp
            FROM check_results
            ORDER BY timestamp DESC
            LIMIT ?1
            "#,
        )?;

        let results = stmt
            .query_map([limit as i64], |row| {
                let policy_name: String = row.get(0)?;
                let passed: i32 = row.get(1)?;
                let message: String = row.get(2)?;
                let affected_str: String = row.get(3)?;
                let timestamp: String = row.get(4)?;

                let affected_beads: Vec<String> = if affected_str.is_empty() {
                    Vec::new()
                } else {
                    affected_str.split(',').map(String::from).collect()
                };

                Ok(CheckResult {
                    policy_name,
                    passed: passed != 0,
                    message,
                    affected_beads,
                    timestamp,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(results)
    }

    /// Get results for a specific policy
    pub fn results_for_policy(&self, policy_name: &str, limit: usize) -> SqlResult<Vec<CheckResult>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT policy_name, passed, message, affected_beads, timestamp
            FROM check_results
            WHERE policy_name = ?1
            ORDER BY timestamp DESC
            LIMIT ?2
            "#,
        )?;

        let results = stmt
            .query_map([policy_name, &limit.to_string()], |row| {
                let policy_name: String = row.get(0)?;
                let passed: i32 = row.get(1)?;
                let message: String = row.get(2)?;
                let affected_str: String = row.get(3)?;
                let timestamp: String = row.get(4)?;

                let affected_beads: Vec<String> = if affected_str.is_empty() {
                    Vec::new()
                } else {
                    affected_str.split(',').map(String::from).collect()
                };

                Ok(CheckResult {
                    policy_name,
                    passed: passed != 0,
                    message,
                    affected_beads,
                    timestamp,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(results)
    }

    /// Delete old check results, keeping only the most recent N per policy
    pub fn cleanup_old_results(&self, keep_per_policy: usize) -> SqlResult<usize> {
        let affected = self.conn.execute(
            r#"
            DELETE FROM check_results
            WHERE id NOT IN (
                SELECT id FROM (
                    SELECT id, ROW_NUMBER() OVER (PARTITION BY policy_name ORDER BY timestamp DESC) as rn
                    FROM check_results
                ) WHERE rn <= ?1
            )
            "#,
            [keep_per_policy as i64],
        )?;
        Ok(affected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_policy() {
        let storage = PolicyStorage::in_memory().unwrap();
        let policy = Policy::new("test-policy", PolicyType::RequireDescription);

        storage.save_policy(&policy).unwrap();
        let loaded = storage.load_policies().unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "test-policy");
    }

    #[test]
    fn test_save_and_load_result() {
        let storage = PolicyStorage::in_memory().unwrap();

        // First save the policy (required by foreign key constraint)
        let policy = Policy::new("test-policy", PolicyType::RequireDescription);
        storage.save_policy(&policy).unwrap();

        // Now save the check result
        let result = CheckResult::fail("test-policy", "Test failure")
            .with_affected_beads(vec!["bead-1".to_string()]);

        storage.save_result(&result).unwrap();
        let loaded = storage.recent_results(10).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].policy_name, "test-policy");
        assert!(!loaded[0].passed);
    }
}
