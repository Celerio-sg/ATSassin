use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct QualityRecord {
    pub call_id: String,
    pub task: String,
    pub provider: String,
    pub model: String,
    pub edit_distance: Option<usize>,
    pub accepted: bool,
    pub confidence_before: f64,
    pub confidence_after: f64,
}

#[derive(Debug, Clone)]
pub struct QualityStats {
    pub total_calls: u64,
    pub accepted: u64,
    pub edited: u64,
    pub ignored: u64,
    pub avg_edit_distance: f64,
    pub acceptance_rate: f64,
    pub edit_rate: f64,
}

impl QualityStats {
    pub fn empty() -> Self {
        Self {
            total_calls: 0,
            accepted: 0,
            edited: 0,
            ignored: 0,
            avg_edit_distance: 0.0,
            acceptance_rate: 0.0,
            edit_rate: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct QualityTracker {
    records: Mutex<Vec<QualityRecord>>,
}

impl QualityTracker {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    pub fn record(&self, record: QualityRecord) {
        self.records.lock().unwrap().push(record);
    }

    pub fn stats_for(&self, task: &str) -> QualityStats {
        let records = self.records.lock().unwrap();
        let filtered: Vec<_> = records.iter().filter(|r| r.task == task).collect();

        if filtered.is_empty() {
            return QualityStats::empty();
        }

        let total = filtered.len() as u64;
        let accepted = filtered
            .iter()
            .filter(|r| r.accepted && r.edit_distance.map(|d| d == 0).unwrap_or(true))
            .count() as u64;
        let edited = filtered
            .iter()
            .filter(|r| r.edit_distance.map(|d| d > 0).unwrap_or(false))
            .count() as u64;
        let ignored = total - accepted - edited;

        let edit_distances: Vec<usize> = filtered.iter().filter_map(|r| r.edit_distance).collect();
        let avg_edit_distance = if edit_distances.is_empty() {
            0.0
        } else {
            edit_distances.iter().sum::<usize>() as f64 / edit_distances.len() as f64
        };

        QualityStats {
            total_calls: total,
            accepted,
            edited,
            ignored,
            avg_edit_distance,
            acceptance_rate: accepted as f64 / total as f64,
            edit_rate: edited as f64 / total as f64,
        }
    }

    pub fn should_escalate(&self, task: &str) -> bool {
        let stats = self.stats_for(task);
        stats.acceptance_rate >= 0.85 && stats.edit_rate <= 0.15 && stats.total_calls >= 10
    }

    pub fn levenstein_distance(a: &str, b: &str) -> usize {
        if a == b {
            return 0;
        }
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let mut dp = vec![vec![0; b_chars.len() + 1]; a_chars.len() + 1];

        for (i, row) in dp.iter_mut().enumerate() {
            row[0] = i;
        }
        for (j, val) in dp[0].iter_mut().enumerate() {
            *val = j;
        }

        for i in 1..=a_chars.len() {
            for j in 1..=b_chars.len() {
                if a_chars[i - 1] == b_chars[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1];
                } else {
                    dp[i][j] = 1 + dp[i - 1][j - 1].min(dp[i - 1][j].min(dp[i][j - 1]));
                }
            }
        }

        dp[a_chars.len()][b_chars.len()]
    }
}

impl Default for QualityTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenstein_distance_identical() {
        assert_eq!(QualityTracker::levenstein_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_levenstein_distance_basic() {
        assert_eq!(QualityTracker::levenstein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_levenstein_distance_empty() {
        assert_eq!(QualityTracker::levenstein_distance("", "abc"), 3);
        assert_eq!(QualityTracker::levenstein_distance("abc", ""), 3);
    }

    #[test]
    fn test_quality_stats_empty() {
        let tracker = QualityTracker::new();
        let stats = tracker.stats_for("scoring");
        assert_eq!(stats.total_calls, 0);
        assert_eq!(stats.acceptance_rate, 0.0);
    }

    #[test]
    fn test_quality_stats_with_records() {
        let tracker = QualityTracker::new();
        tracker.record(QualityRecord {
            call_id: "1".into(),
            task: "scoring".into(),
            provider: "groq".into(),
            model: "llama".into(),
            edit_distance: Some(0),
            accepted: true,
            confidence_before: 0.8,
            confidence_after: 0.9,
        });
        let stats = tracker.stats_for("scoring");
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.accepted, 1);
        assert_eq!(stats.acceptance_rate, 1.0);
        assert_eq!(stats.edit_rate, 0.0);
    }

    #[test]
    fn test_should_escalate_gate() {
        let tracker = QualityTracker::new();
        for i in 0..10 {
            tracker.record(QualityRecord {
                call_id: format!("{}", i),
                task: "scoring".into(),
                provider: "groq".into(),
                model: "llama".into(),
                edit_distance: if i < 9 { Some(0) } else { Some(3) },
                accepted: true,
                confidence_before: 0.8,
                confidence_after: 0.9,
            });
        }
        assert!(tracker.should_escalate("scoring"));
    }
}
