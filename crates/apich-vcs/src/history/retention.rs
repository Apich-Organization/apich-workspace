use crate::model::Snapshot;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

/// Timeline retention policy utilizing exponential decay (Grandfather-Father-Son)
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    /// Time window where ALL micro-revisions are preserved (default: 24 hours)
    pub keep_all_duration: Duration,
    /// Time window where snapshots are reduced to 1 per active hour (default: 7 days)
    pub hourly_duration: Duration,
    /// Time window where snapshots are reduced to 1 per active day (default: 30 days)
    pub daily_duration: Duration,
    /// Time window where snapshots are reduced to 1 per active week (default: 365 days)
    pub weekly_duration: Duration,
    /// Beyond 1 year, preserve 1 snapshot per active month forever
    pub monthly_beyond: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_all_duration: Duration::hours(24),
            hourly_duration: Duration::days(7),
            daily_duration: Duration::days(30),
            weekly_duration: Duration::days(365),
            monthly_beyond: true,
        }
    }
}

impl RetentionPolicy {
    /// Strict academic policy preserving all snapshots indefinitely
    pub fn keep_all() -> Self {
        Self {
            keep_all_duration: Duration::weeks(5200), // ~100 years
            hourly_duration: Duration::weeks(5200),
            daily_duration: Duration::weeks(5200),
            weekly_duration: Duration::weeks(5200),
            monthly_beyond: true,
        }
    }

    /// Determine which snapshot IDs should be pruned given the current list and reference time
    pub fn select_snapshots_to_prune(
        &self,
        snapshots: &[Snapshot],
        now: DateTime<Utc>,
    ) -> Vec<Uuid> {
        if snapshots.is_empty() {
            return Vec::new();
        }

        let mut keep = HashSet::new();

        // 1. ALWAYS keep all Milestones
        for s in snapshots {
            if s.is_milestone {
                keep.insert(s.id);
            }
        }

        // 2. ALWAYS keep snapshots with Git mapping
        for s in snapshots {
            if s.git_commit_oid.is_some() {
                keep.insert(s.id);
            }
        }

        // 3. ALWAYS keep the newest snapshot (HEAD)
        if let Some(latest) = snapshots.last() {
            keep.insert(latest.id);
        }

        // Buckets for historical snapshots
        let mut hourly_buckets: BTreeMap<(i32, u32, u32, u32), Uuid> = BTreeMap::new();
        let mut daily_buckets: BTreeMap<(i32, u32, u32), Uuid> = BTreeMap::new();
        let mut weekly_buckets: BTreeMap<(i32, u32), Uuid> = BTreeMap::new();
        let mut monthly_buckets: BTreeMap<(i32, u32), Uuid> = BTreeMap::new();

        for s in snapshots {
            let age = now.signed_duration_since(s.created_at);

            if age <= self.keep_all_duration {
                // Tier 1: 0 - 24 hours -> Keep all
                keep.insert(s.id);
            } else if age <= self.hourly_duration {
                // Tier 2: 1 - 7 days -> 1 per active hour (keep the latest in that hour)
                let key = (
                    s.created_at.year(),
                    s.created_at.month(),
                    s.created_at.day(),
                    s.created_at.hour(),
                );
                hourly_buckets.insert(key, s.id);
            } else if age <= self.daily_duration {
                // Tier 3: 8 - 30 days -> 1 per active day
                let key = (
                    s.created_at.year(),
                    s.created_at.month(),
                    s.created_at.day(),
                );
                daily_buckets.insert(key, s.id);
            } else if age <= self.weekly_duration {
                // Tier 4: 30 - 365 days -> 1 per active week
                let key = (
                    s.created_at.year(),
                    s.created_at.iso_week().week(),
                );
                weekly_buckets.insert(key, s.id);
            } else if self.monthly_beyond {
                // Tier 5: 1+ years -> 1 per active month forever
                let key = (
                    s.created_at.year(),
                    s.created_at.month(),
                );
                monthly_buckets.insert(key, s.id);
            }
        }

        // Add representatives from each bucket
        for id in hourly_buckets.values() {
            keep.insert(*id);
        }
        for id in daily_buckets.values() {
            keep.insert(*id);
        }
        for id in weekly_buckets.values() {
            keep.insert(*id);
        }
        for id in monthly_buckets.values() {
            keep.insert(*id);
        }

        // Return snapshots not in the keep set
        snapshots
            .iter()
            .filter(|s| !keep.contains(&s.id))
            .map(|s| s.id)
            .collect()
    }
}
