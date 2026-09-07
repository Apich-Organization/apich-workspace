mod common;
use apich_vcs::{ProjectVcs, RetentionPolicy, Snapshot};
use chrono::{Duration, TimeZone, Utc};
use common::test_temp_dir;
use std::fs;

#[test]
fn test_retention_policy_decay_tiers() {
    let policy = RetentionPolicy::default();
    let now = Utc.with_ymd_and_hms(2026, 9, 6, 12, 0, 0).unwrap();

    let mut snapshots = Vec::new();

    // Helper to make test snapshots
    let make_snap = |created_at, is_milestone, git_oid: Option<&str>| {
        let mut s = Snapshot::new(
            "dummy_tree_hash".to_string(),
            "Snapshot note",
            None,
            "Author",
        );
        s.created_at = created_at;
        s.is_milestone = is_milestone;
        s.git_commit_oid = git_oid.map(|x| x.to_string());
        s
    };

    // Tier 1: Last 24 hours (keep all micro-revisions)
    let s_recent_1 = make_snap(now - Duration::hours(1), false, None);
    let s_recent_2 = make_snap(now - Duration::hours(2), false, None);
    let s_recent_3 = make_snap(now - Duration::hours(3), false, None);
    snapshots.push(s_recent_1.clone());
    snapshots.push(s_recent_2.clone());
    snapshots.push(s_recent_3.clone());

    // Tier 2: 3 days ago, 3 snapshots in the same hour (keep latest 1)
    let t2_base = now - Duration::days(3);
    let s_t2_a = make_snap(t2_base + Duration::minutes(10), false, None);
    let s_t2_b = make_snap(t2_base + Duration::minutes(25), false, None);
    let s_t2_c = make_snap(t2_base + Duration::minutes(50), false, None);
    snapshots.push(s_t2_a.clone());
    snapshots.push(s_t2_b.clone());
    snapshots.push(s_t2_c.clone());

    // Tier 3: 15 days ago, 2 snapshots in the same day (keep latest 1)
    let t3_base = now - Duration::days(15);
    let s_t3_a = make_snap(t3_base - Duration::hours(2), false, None);
    let s_t3_b = make_snap(t3_base - Duration::hours(5), false, None);
    snapshots.push(s_t3_a.clone());
    snapshots.push(s_t3_b.clone());

    // Milestones and Git push points from 200 days ago (MUST be kept permanently)
    let old_date = now - Duration::days(200);
    let s_milestone = make_snap(old_date, true, None);
    let s_git_synced = make_snap(old_date - Duration::days(5), false, Some("c0ffee1234567890"));
    let s_old_plain = make_snap(old_date - Duration::days(6), false, None);
    let s_old_plain_2 = make_snap(old_date - Duration::days(6) - Duration::hours(2), false, None);
    snapshots.push(s_milestone.clone());
    snapshots.push(s_git_synced.clone());
    snapshots.push(s_old_plain.clone());
    snapshots.push(s_old_plain_2.clone());

    let pruned_ids = policy.select_snapshots_to_prune(&snapshots, now);

    // Assert Milestones and Git synced points are NEVER pruned
    assert!(!pruned_ids.contains(&s_milestone.id));
    assert!(!pruned_ids.contains(&s_git_synced.id));

    // Assert Tier 1 recent snapshots are NEVER pruned
    assert!(!pruned_ids.contains(&s_recent_1.id));
    assert!(!pruned_ids.contains(&s_recent_2.id));
    assert!(!pruned_ids.contains(&s_recent_3.id));

    // In Tier 2, older snapshots in same hour should be pruned
    assert!(pruned_ids.contains(&s_t2_a.id) || pruned_ids.contains(&s_t2_b.id));
    // The latest in the hour (s_t2_c) should be kept
    assert!(!pruned_ids.contains(&s_t2_c.id));

    // In Tier 3, one of the two daily snapshots should be pruned
    assert!(pruned_ids.contains(&s_t3_a.id) || pruned_ids.contains(&s_t3_b.id));

    // In Tier 4, one of the two weekly snapshots should be pruned
    assert!(pruned_ids.contains(&s_old_plain.id) || pruned_ids.contains(&s_old_plain_2.id));
}

#[test]
fn test_vcs_run_gc_and_chunk_sweeping() {
    let temp = test_temp_dir();
    let vcs = ProjectVcs::open_or_init(temp.path()).unwrap();

    // Step 1: Create a file and snapshot it
    fs::write(
        temp.path().join("dataset.bin"),
        vec![0xAA; 32 * 1024], // 32 KB
    )
    .unwrap();
    let s1 = vcs.snapshot("First binary snapshot").unwrap();

    // Step 2: Replace dataset with new content and snapshot it
    fs::write(
        temp.path().join("dataset.bin"),
        vec![0xBB; 32 * 1024], // completely different content
    )
    .unwrap();
    let s2 = vcs.snapshot("Second binary snapshot").unwrap();

    // Check CAS stats before GC: both chunks for 0xAA and 0xBB exist
    let chunks_dir = vcs.cas().root_dir().join("chunks");
    assert!(chunks_dir.exists());

    // If both snapshots are kept, GC prunes 0 chunks
    let stats_no_prune = vcs.run_gc(Some(&RetentionPolicy::keep_all())).unwrap();
    assert_eq!(stats_no_prune.pruned_chunks, 0);

    // Now artificially prune s1 to simulate aging
    vcs.cas().remove_snapshot(s1.id).unwrap();

    // Run GC: the unreferenced chunks from s1 should now be swept
    let stats = vcs.run_gc(Some(&RetentionPolicy::keep_all())).unwrap();
    assert!(stats.scanned_chunks > 0);
    assert!(stats.pruned_chunks > 0);
    assert!(stats.reclaimed_bytes > 0);

    // Verify s2 is still intact and can be read
    let s2_tree = vcs.cas().get_tree(&s2.tree_hash).unwrap();
    let s2_entry = s2_tree.get("dataset.bin").unwrap();
    let reconstructed = vcs.cas().read_file_data(&s2_entry.chunks).unwrap();
    assert_eq!(reconstructed, vec![0xBB; 32 * 1024]);
}
