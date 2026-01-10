//! Integration tests for AllBeads
//!
//! These tests verify the full workflow from config loading through aggregation.

use allbeads::cache::{Cache, CacheConfig};
use allbeads::config::{AllBeadsConfig, AuthStrategy, BossContext};
use allbeads::graph::{Bead, BeadId, FederatedGraph, IssueType, Priority, Status};
use chrono::Utc;
use std::collections::HashSet;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a test bead
fn create_test_bead(id: &str, title: &str, status: Status, priority: Priority) -> Bead {
    let mut labels = HashSet::new();
    labels.insert("@test".to_string());

    Bead {
        id: BeadId::new(id),
        title: title.to_string(),
        description: None,
        status,
        priority,
        issue_type: IssueType::Task,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        created_by: "test".to_string(),
        assignee: None,
        labels,
        dependencies: vec![],
        blocks: vec![],
        notes: None,
    }
}

mod config_tests {
    use super::*;

    #[test]
    fn test_config_creation_and_save() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a new config
        let mut config = AllBeadsConfig::new();

        let context = BossContext::new(
            "test-project",
            "https://github.com/test/repo.git",
            AuthStrategy::SshAgent,
        );
        config.add_context(context);

        // Save and reload
        config.save(&config_path).unwrap();

        let loaded = AllBeadsConfig::load(&config_path).unwrap();
        assert_eq!(loaded.contexts.len(), 1);
        assert_eq!(loaded.contexts[0].name, "test-project");
    }

    #[test]
    fn test_config_multiple_contexts() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let mut config = AllBeadsConfig::new();

        config.add_context(BossContext::new(
            "work",
            "git@github.com:org/work.git",
            AuthStrategy::SshAgent,
        ));
        config.add_context(BossContext::new(
            "personal",
            "git@github.com:user/personal.git",
            AuthStrategy::SshAgent,
        ));
        config.add_context(BossContext::new(
            "oss",
            "https://github.com/oss/project.git",
            AuthStrategy::PersonalAccessToken,
        ));

        config.save(&config_path).unwrap();

        let loaded = AllBeadsConfig::load(&config_path).unwrap();
        assert_eq!(loaded.contexts.len(), 3);

        // Verify context lookup
        assert!(loaded.get_context("work").is_some());
        assert!(loaded.get_context("personal").is_some());
        assert!(loaded.get_context("oss").is_some());
        assert!(loaded.get_context("nonexistent").is_none());
    }

    #[test]
    fn test_context_remove() {
        let mut config = AllBeadsConfig::new();
        config.add_context(BossContext::new("a", "url-a", AuthStrategy::SshAgent));
        config.add_context(BossContext::new("b", "url-b", AuthStrategy::SshAgent));

        assert_eq!(config.contexts.len(), 2);

        let removed = config.remove_context("a");
        assert!(removed.is_some());
        assert_eq!(config.contexts.len(), 1);
        assert!(config.get_context("a").is_none());
        assert!(config.get_context("b").is_some());
    }
}

mod graph_construction_tests {
    use super::*;

    #[test]
    fn test_graph_from_beads() {
        let mut graph = FederatedGraph::new();

        let beads = vec![
            create_test_bead("test-1", "First Task", Status::Open, Priority::P1),
            create_test_bead("test-2", "Second Task", Status::InProgress, Priority::P2),
            create_test_bead("test-3", "Third Task", Status::Closed, Priority::P3),
        ];

        for bead in beads {
            graph.add_bead(bead);
        }

        assert_eq!(graph.beads.len(), 3);

        // Verify bead data
        let first = graph.get_bead(&BeadId::new("test-1")).unwrap();
        assert_eq!(first.title, "First Task");
        assert_eq!(first.status, Status::Open);
        assert_eq!(first.priority, Priority::P1);
    }

    #[test]
    fn test_graph_with_labels() {
        let mut graph = FederatedGraph::new();

        let mut bead = create_test_bead("labeled", "Labeled Task", Status::Open, Priority::P2);
        bead.labels.insert("@work".to_string());
        bead.labels.insert("urgent".to_string());
        bead.labels.insert("backend".to_string());

        graph.add_bead(bead);

        let loaded = graph.get_bead(&BeadId::new("labeled")).unwrap();
        assert_eq!(loaded.labels.len(), 4); // @test + 3 more
        assert!(loaded.labels.contains("@work"));
    }
}

mod cache_tests {
    use super::*;

    #[test]
    fn test_cache_store_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache_config = CacheConfig {
            path: temp_dir.path().join("cache.db"),
            ttl: Duration::from_secs(300),
            wal_mode: false,
        };

        let cache = Cache::new(cache_config).unwrap();

        // Create a graph with some beads
        let mut graph = FederatedGraph::new();
        graph.add_bead(create_test_bead(
            "cache-1",
            "Cached Task",
            Status::Open,
            Priority::P1,
        ));
        graph.add_bead(create_test_bead(
            "cache-2",
            "Another Task",
            Status::Closed,
            Priority::P2,
        ));

        // Store in cache
        cache.store_graph(&graph).unwrap();

        // Load from cache
        let loaded = cache.load_graph().unwrap();
        assert!(loaded.is_some());

        let loaded_graph = loaded.unwrap();
        assert_eq!(loaded_graph.beads.len(), 2);
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache_config = CacheConfig {
            path: temp_dir.path().join("cache.db"),
            ttl: Duration::from_secs(300),
            wal_mode: false,
        };

        let cache = Cache::new(cache_config).unwrap();

        let mut graph = FederatedGraph::new();
        graph.add_bead(create_test_bead(
            "clear-1",
            "Task to Clear",
            Status::Open,
            Priority::P1,
        ));

        cache.store_graph(&graph).unwrap();
        assert!(cache.load_graph().unwrap().is_some());

        cache.clear().unwrap();
        assert!(cache.load_graph().unwrap().is_none());
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cache_config = CacheConfig {
            path: temp_dir.path().join("cache.db"),
            ttl: Duration::from_secs(300),
            wal_mode: false,
        };

        let cache = Cache::new(cache_config).unwrap();

        let mut graph = FederatedGraph::new();
        graph.add_bead(create_test_bead(
            "stats-1",
            "Task 1",
            Status::Open,
            Priority::P1,
        ));
        graph.add_bead(create_test_bead(
            "stats-2",
            "Task 2",
            Status::Closed,
            Priority::P2,
        ));

        cache.store_graph(&graph).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.bead_count, 2);
        assert!(!stats.is_expired);
    }
}

mod graph_tests {
    use super::*;

    #[test]
    fn test_ready_beads() {
        let mut graph = FederatedGraph::new();

        // Open bead with no dependencies - should be ready
        graph.add_bead(create_test_bead(
            "ready-1",
            "Ready Task",
            Status::Open,
            Priority::P1,
        ));

        // Open bead with dependencies - not ready (dependency doesn't exist, treated as blocked)
        let mut blocked = create_test_bead("blocked-1", "Blocked Task", Status::Open, Priority::P2);
        blocked.dependencies = vec![BeadId::new("missing-dep")];
        graph.add_bead(blocked);

        // Closed bead - not ready
        graph.add_bead(create_test_bead(
            "closed-1",
            "Closed Task",
            Status::Closed,
            Priority::P3,
        ));

        let ready = graph.ready_beads();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].id.as_str(), "ready-1");
    }

    #[test]
    fn test_graph_stats() {
        let mut graph = FederatedGraph::new();

        graph.add_bead(create_test_bead(
            "open-1",
            "Open 1",
            Status::Open,
            Priority::P1,
        ));
        graph.add_bead(create_test_bead(
            "open-2",
            "Open 2",
            Status::Open,
            Priority::P2,
        ));
        graph.add_bead(create_test_bead(
            "progress-1",
            "In Progress",
            Status::InProgress,
            Priority::P1,
        ));
        graph.add_bead(create_test_bead(
            "closed-1",
            "Closed",
            Status::Closed,
            Priority::P3,
        ));
        graph.add_bead(create_test_bead(
            "closed-2",
            "Closed 2",
            Status::Closed,
            Priority::P4,
        ));

        let stats = graph.stats();
        assert_eq!(stats.total_beads, 5);
        assert_eq!(stats.open_beads, 2);
        assert_eq!(stats.in_progress_beads, 1);
        assert_eq!(stats.closed_beads, 2);
    }

    #[test]
    fn test_bead_lookup() {
        let mut graph = FederatedGraph::new();

        graph.add_bead(create_test_bead(
            "lookup-1",
            "Task to Find",
            Status::Open,
            Priority::P1,
        ));
        graph.add_bead(create_test_bead(
            "lookup-2",
            "Another Task",
            Status::Open,
            Priority::P2,
        ));

        let found = graph.get_bead(&BeadId::new("lookup-1"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Task to Find");

        let not_found = graph.get_bead(&BeadId::new("nonexistent"));
        assert!(not_found.is_none());
    }
}

mod mail_tests {
    use allbeads::mail::*;
    use std::time::Duration;

    #[test]
    fn test_lock_message_roundtrip() {
        let msg = Message::from_strings(
            "worker@project",
            "postmaster@project",
            MessageType::Lock(
                LockRequest::new("src/main.rs", Duration::from_secs(3600))
                    .with_reason("Refactoring main entry point"),
            ),
        );

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.from.to_string(), "worker@project");
        assert!(parsed.is_lock());
    }

    #[test]
    fn test_human_inbox_message() {
        let msg = Message::from_strings(
            "bot@project",
            "human@localhost",
            MessageType::Request(RequestPayload::new("Approve deployment?")),
        );

        assert!(msg.is_for_human());
        assert!(!msg.is_broadcast());
    }

    #[test]
    fn test_broadcast_message() {
        let msg = Message::from_strings(
            "monitor@project",
            "all@project",
            MessageType::Broadcast(
                BroadcastPayload::new("API rate limit reached")
                    .with_category(BroadcastCategory::RateLimit),
            ),
        );

        assert!(msg.is_broadcast());
        assert!(!msg.is_for_human());
    }

    #[test]
    fn test_address_parsing() {
        let addr: Address = "agent@project".parse().unwrap();
        assert_eq!(addr.name(), "agent");
        assert_eq!(addr.domain(), "project");
    }

    #[test]
    fn test_special_addresses() {
        let human = Address::human();
        assert!(human.is_human());

        let broadcast = Address::broadcast("my-project");
        assert!(broadcast.is_broadcast());
        assert!(broadcast.is_in_project("my-project"));

        let postmaster = Address::postmaster("my-project");
        assert!(postmaster.is_postmaster());
    }

    #[test]
    fn test_routing_target() {
        let human = Address::human();
        assert!(matches!(
            RoutingTarget::from_address(&human),
            RoutingTarget::Human
        ));

        let broadcast = Address::broadcast("proj");
        assert!(matches!(
            RoutingTarget::from_address(&broadcast),
            RoutingTarget::Broadcast { .. }
        ));
    }

    #[test]
    fn test_lock_acquire_and_release() {
        let mut manager = LockManager::new();
        let holder: Address = "worker@project".parse().unwrap();

        // Acquire lock
        let result = manager.acquire(
            "src/main.rs",
            holder.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );
        assert!(matches!(result, LockResult::Acquired { .. }));

        // Check status
        let status = manager.status("src/main.rs");
        assert!(status.is_some());
        assert_eq!(status.unwrap().holder, holder);

        // Release
        let result = manager.release("src/main.rs", &holder);
        assert!(matches!(result, LockResult::Released));

        // Verify released
        assert!(manager.status("src/main.rs").is_none());
    }

    #[test]
    fn test_lock_conflict() {
        let mut manager = LockManager::new();
        let holder1: Address = "worker1@project".parse().unwrap();
        let holder2: Address = "worker2@project".parse().unwrap();

        // First agent acquires lock
        manager.acquire(
            "src/main.rs",
            holder1.clone(),
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );

        // Second agent tries to acquire - should fail
        let result = manager.acquire(
            "src/main.rs",
            holder2,
            Duration::from_secs(3600),
            ConflictStrategy::Abort,
        );
        assert!(matches!(result, LockResult::Denied { holder, .. } if holder == holder1));
    }

    #[test]
    fn test_postmaster_send_receive() {
        use allbeads::mail::{Postmaster, SendResult};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("mail.db");
        let mut postmaster = Postmaster::new(db_path).unwrap();

        // Send a message
        let msg = Message::from_strings(
            "worker@project",
            "human@localhost",
            MessageType::Notify(NotifyPayload::new("Task completed")),
        );
        let result = postmaster.send(msg).unwrap();
        assert!(matches!(result, SendResult::Delivered { .. }));

        // Check inbox
        let human = Address::human();
        let inbox = postmaster.inbox(&human).unwrap();
        assert_eq!(inbox.len(), 1);
    }

    #[test]
    fn test_postmaster_lock_via_message() {
        use allbeads::mail::{Postmaster, SendResult};
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("mail.db");
        let mut postmaster = Postmaster::with_project_id(db_path, "test").unwrap();

        // Send lock request
        let msg = Message::from_strings(
            "worker@test",
            "postmaster@test",
            MessageType::Lock(LockRequest::new("src/main.rs", Duration::from_secs(3600))),
        );
        let result = postmaster.send(msg).unwrap();

        // Should get lock
        assert!(matches!(
            result,
            SendResult::LockResult {
                result: LockResult::Acquired { .. },
                ..
            }
        ));

        // Verify lock exists
        assert!(postmaster.lock_manager().status("src/main.rs").is_some());
    }
}
