//! File locking protocol for Agent Mail
//!
//! Provides mutex-style file locking to prevent concurrent modifications.
//!
//! # Overview
//!
//! The lock manager tracks exclusive file locks with TTLs:
//! - Agents request locks via LOCK messages
//! - Locks expire after TTL to prevent deadlocks
//! - Conflict resolution strategies: WAIT, STEAL, ABORT
//!
//! # Example
//!
//! ```no_run
//! use allbeads::mail::{Address, LockManager, LockResult, ConflictStrategy};
//! use std::time::Duration;
//!
//! let mut manager = LockManager::new();
//!
//! let holder: Address = "worker@project".parse().unwrap();
//! let result = manager.acquire(
//!     "src/main.rs",
//!     holder,
//!     Duration::from_secs(3600),
//!     ConflictStrategy::Abort,
//! );
//!
//! match result {
//!     LockResult::Acquired { expires_at } => println!("Got lock until {:?}", expires_at),
//!     LockResult::Denied { holder, .. } => println!("Locked by {}", holder),
//!     LockResult::Released => println!("Lock released"),
//!     _ => {}
//! }
//! ```

use super::Address;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Result of a lock operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockResult {
    /// Lock was successfully acquired
    Acquired {
        /// When the lock expires
        expires_at: DateTime<Utc>,
    },

    /// Lock was denied because another agent holds it
    Denied {
        /// Current lock holder
        holder: Address,
        /// When the current lock expires
        expires_at: DateTime<Utc>,
        /// Optional reason for the lock
        reason: Option<String>,
    },

    /// Lock was successfully released
    Released,

    /// The file was not locked
    NotLocked,

    /// Lock was stolen from another agent
    Stolen {
        /// Previous lock holder
        previous_holder: Address,
        /// New expiration time
        expires_at: DateTime<Utc>,
    },
}

/// Strategy for handling lock conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// Fail immediately if lock is held
    #[default]
    Abort,

    /// Wait for lock to be released (not yet implemented)
    Wait,

    /// Forcibly take the lock (requires authorization)
    Steal,
}

/// Information about an active lock
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockInfo {
    /// File path being locked
    pub path: String,

    /// Lock holder address
    pub holder: Address,

    /// When the lock was acquired
    pub acquired_at: DateTime<Utc>,

    /// When the lock expires
    pub expires_at: DateTime<Utc>,

    /// Optional reason for the lock
    pub reason: Option<String>,
}

impl LockInfo {
    /// Check if this lock has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Get remaining TTL
    pub fn remaining_ttl(&self) -> Option<Duration> {
        let now = Utc::now();
        if now > self.expires_at {
            None
        } else {
            let remaining = self.expires_at - now;
            Some(Duration::from_secs(remaining.num_seconds().max(0) as u64))
        }
    }
}

/// In-memory lock manager
///
/// For production use, this should be backed by SQLite for persistence.
#[derive(Debug, Default)]
pub struct LockManager {
    /// Active locks by file path
    locks: HashMap<String, LockInfo>,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to acquire a lock on a file
    pub fn acquire(
        &mut self,
        path: impl Into<String>,
        holder: Address,
        ttl: Duration,
        strategy: ConflictStrategy,
    ) -> LockResult {
        self.acquire_with_reason(path, holder, ttl, strategy, None)
    }

    /// Try to acquire a lock with a reason
    pub fn acquire_with_reason(
        &mut self,
        path: impl Into<String>,
        holder: Address,
        ttl: Duration,
        strategy: ConflictStrategy,
        reason: Option<String>,
    ) -> LockResult {
        let path = path.into();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(ttl.as_secs() as i64);

        // Check for existing lock
        if let Some(existing) = self.locks.get(&path) {
            // Check if expired
            if existing.is_expired() {
                // Lock expired, we can take it
                let lock_info = LockInfo {
                    path: path.clone(),
                    holder,
                    acquired_at: now,
                    expires_at,
                    reason,
                };
                self.locks.insert(path, lock_info);
                return LockResult::Acquired { expires_at };
            }

            // Lock is still valid - check if same holder (renewal)
            if existing.holder == holder {
                // Renewal - update expiration
                let lock_info = LockInfo {
                    path: path.clone(),
                    holder,
                    acquired_at: existing.acquired_at,
                    expires_at,
                    reason: reason.or_else(|| existing.reason.clone()),
                };
                self.locks.insert(path, lock_info);
                return LockResult::Acquired { expires_at };
            }

            // Different holder - apply conflict strategy
            match strategy {
                ConflictStrategy::Abort => {
                    return LockResult::Denied {
                        holder: existing.holder.clone(),
                        expires_at: existing.expires_at,
                        reason: existing.reason.clone(),
                    };
                }
                ConflictStrategy::Wait => {
                    // Wait strategy not implemented yet - treat as abort
                    return LockResult::Denied {
                        holder: existing.holder.clone(),
                        expires_at: existing.expires_at,
                        reason: existing.reason.clone(),
                    };
                }
                ConflictStrategy::Steal => {
                    let previous_holder = existing.holder.clone();
                    let lock_info = LockInfo {
                        path: path.clone(),
                        holder,
                        acquired_at: now,
                        expires_at,
                        reason,
                    };
                    self.locks.insert(path, lock_info);
                    return LockResult::Stolen {
                        previous_holder,
                        expires_at,
                    };
                }
            }
        }

        // No existing lock - acquire it
        let lock_info = LockInfo {
            path: path.clone(),
            holder,
            acquired_at: now,
            expires_at,
            reason,
        };
        self.locks.insert(path, lock_info);
        LockResult::Acquired { expires_at }
    }

    /// Release a lock
    ///
    /// Only the lock holder can release their own lock.
    pub fn release(&mut self, path: &str, holder: &Address) -> LockResult {
        if let Some(existing) = self.locks.get(path) {
            if existing.is_expired() {
                self.locks.remove(path);
                return LockResult::NotLocked;
            }

            if &existing.holder == holder {
                self.locks.remove(path);
                return LockResult::Released;
            }

            // Someone else holds the lock
            return LockResult::Denied {
                holder: existing.holder.clone(),
                expires_at: existing.expires_at,
                reason: existing.reason.clone(),
            };
        }

        LockResult::NotLocked
    }

    /// Force release a lock (admin operation)
    pub fn force_release(&mut self, path: &str) -> LockResult {
        if self.locks.remove(path).is_some() {
            LockResult::Released
        } else {
            LockResult::NotLocked
        }
    }

    /// Check the status of a lock
    pub fn status(&self, path: &str) -> Option<&LockInfo> {
        self.locks.get(path).filter(|info| !info.is_expired())
    }

    /// List all active (non-expired) locks
    pub fn active_locks(&self) -> Vec<&LockInfo> {
        self.locks
            .values()
            .filter(|info| !info.is_expired())
            .collect()
    }

    /// List all locks held by a specific agent
    pub fn locks_by_holder(&self, holder: &Address) -> Vec<&LockInfo> {
        self.locks
            .values()
            .filter(|info| !info.is_expired() && &info.holder == holder)
            .collect()
    }

    /// Clean up expired locks
    pub fn cleanup_expired(&mut self) -> usize {
        let expired: Vec<String> = self
            .locks
            .iter()
            .filter(|(_, info)| info.is_expired())
            .map(|(path, _)| path.clone())
            .collect();

        let count = expired.len();
        for path in expired {
            self.locks.remove(&path);
        }
        count
    }

    /// Get the number of active locks
    pub fn lock_count(&self) -> usize {
        self.locks
            .values()
            .filter(|info| !info.is_expired())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address(name: &str) -> Address {
        format!("{}@project", name).parse().unwrap()
    }

    #[test]
    fn test_acquire_lock() {
        let mut manager = LockManager::new();
        let holder = test_address("worker");

        let result = manager.acquire(
            "src/main.rs",
            holder,
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );

        assert!(matches!(result, LockResult::Acquired { .. }));
        assert_eq!(manager.lock_count(), 1);
    }

    #[test]
    fn test_lock_conflict_abort() {
        let mut manager = LockManager::new();
        let holder1 = test_address("worker1");
        let holder2 = test_address("worker2");

        // First lock succeeds
        let result1 = manager.acquire(
            "src/main.rs",
            holder1.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );
        assert!(matches!(result1, LockResult::Acquired { .. }));

        // Second lock fails
        let result2 = manager.acquire(
            "src/main.rs",
            holder2,
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );
        assert!(matches!(result2, LockResult::Denied { holder, .. } if holder == holder1));
    }

    #[test]
    fn test_lock_conflict_steal() {
        let mut manager = LockManager::new();
        let holder1 = test_address("worker1");
        let holder2 = test_address("worker2");

        // First lock
        manager.acquire(
            "src/main.rs",
            holder1.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );

        // Steal the lock
        let result = manager.acquire(
            "src/main.rs",
            holder2,
            Duration::from_secs(3600),
            ConflictStrategy::Steal,
        );
        assert!(
            matches!(result, LockResult::Stolen { previous_holder, .. } if previous_holder == holder1)
        );
    }

    #[test]
    fn test_lock_renewal() {
        let mut manager = LockManager::new();
        let holder = test_address("worker");

        // Initial lock
        manager.acquire(
            "src/main.rs",
            holder.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );

        // Renew with longer TTL
        let result = manager.acquire(
            "src/main.rs",
            holder,
            Duration::from_secs(7200),
            ConflictStrategy::Abort,
        );
        assert!(matches!(result, LockResult::Acquired { .. }));
    }

    #[test]
    fn test_release_lock() {
        let mut manager = LockManager::new();
        let holder = test_address("worker");

        // Acquire
        manager.acquire(
            "src/main.rs",
            holder.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );

        // Release
        let result = manager.release("src/main.rs", &holder);
        assert!(matches!(result, LockResult::Released));
        assert_eq!(manager.lock_count(), 0);
    }

    #[test]
    fn test_release_wrong_holder() {
        let mut manager = LockManager::new();
        let holder1 = test_address("worker1");
        let holder2 = test_address("worker2");

        // Acquire as holder1
        manager.acquire(
            "src/main.rs",
            holder1.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );

        // Try to release as holder2
        let result = manager.release("src/main.rs", &holder2);
        assert!(matches!(result, LockResult::Denied { holder, .. } if holder == holder1));
    }

    #[test]
    fn test_lock_status() {
        let mut manager = LockManager::new();
        let holder = test_address("worker");

        assert!(manager.status("src/main.rs").is_none());

        manager.acquire_with_reason(
            "src/main.rs",
            holder.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
            Some("Refactoring".to_string()),
        );

        let status = manager.status("src/main.rs").unwrap();
        assert_eq!(status.holder, holder);
        assert_eq!(status.reason, Some("Refactoring".to_string()));
    }

    #[test]
    fn test_locks_by_holder() {
        let mut manager = LockManager::new();
        let holder = test_address("worker");

        manager.acquire(
            "file1.rs",
            holder.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );
        manager.acquire(
            "file2.rs",
            holder.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );

        let locks = manager.locks_by_holder(&holder);
        assert_eq!(locks.len(), 2);
    }

    #[test]
    fn test_force_release() {
        let mut manager = LockManager::new();
        let holder = test_address("worker");

        manager.acquire(
            "src/main.rs",
            holder,
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );

        let result = manager.force_release("src/main.rs");
        assert!(matches!(result, LockResult::Released));
        assert_eq!(manager.lock_count(), 0);
    }
}
