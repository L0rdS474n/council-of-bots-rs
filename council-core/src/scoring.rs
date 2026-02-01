//! Score tracking for the simulation.

/// Tracks cumulative score throughout the simulation.
#[derive(Debug, Clone, Default)]
pub struct ScoreTracker {
    /// Total accumulated score.
    pub total: i32,
    /// History of score changes.
    pub history: Vec<ScoreEvent>,
}

/// A single score change event.
#[derive(Debug, Clone)]
pub struct ScoreEvent {
    /// Round when this occurred.
    pub round: u32,
    /// Points gained or lost.
    pub delta: i32,
    /// Reason for the change.
    pub reason: String,
}

impl ScoreTracker {
    /// Create a new score tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a score change.
    pub fn add(&mut self, round: u32, delta: i32, reason: &str) {
        self.total += delta;
        self.history.push(ScoreEvent {
            round,
            delta,
            reason: reason.to_string(),
        });
    }

    /// Get the rating based on total score (for a 25-round game).
    pub fn rating(&self) -> &'static str {
        match self.total {
            200.. => "Legendary Council",
            150..=199 => "Distinguished",
            100..=149 => "Competent",
            50..=99 => "Struggling",
            _ => "Dysfunctional",
        }
    }

    /// Find the best moment (highest single delta).
    pub fn best_moment(&self) -> Option<&ScoreEvent> {
        self.history.iter().max_by_key(|e| e.delta)
    }

    /// Find the worst moment (lowest single delta).
    pub fn worst_moment(&self) -> Option<&ScoreEvent> {
        self.history.iter().min_by_key(|e| e.delta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_starts_at_zero() {
        let tracker = ScoreTracker::new();
        assert_eq!(tracker.total, 0);
        assert!(tracker.history.is_empty());
    }

    #[test]
    fn add_accumulates_score() {
        let mut tracker = ScoreTracker::new();
        tracker.add(1, 10, "Good choice");
        tracker.add(2, -5, "Bad choice");
        assert_eq!(tracker.total, 5);
        assert_eq!(tracker.history.len(), 2);
    }

    #[test]
    fn rating_thresholds() {
        let mut tracker = ScoreTracker::new();

        tracker.total = 250;
        assert_eq!(tracker.rating(), "Legendary Council");

        tracker.total = 175;
        assert_eq!(tracker.rating(), "Distinguished");

        tracker.total = 120;
        assert_eq!(tracker.rating(), "Competent");

        tracker.total = 75;
        assert_eq!(tracker.rating(), "Struggling");

        tracker.total = 25;
        assert_eq!(tracker.rating(), "Dysfunctional");
    }

    #[test]
    fn best_and_worst_moments() {
        let mut tracker = ScoreTracker::new();
        tracker.add(1, 10, "Good");
        tracker.add(2, -15, "Bad");
        tracker.add(3, 5, "Okay");

        assert_eq!(tracker.best_moment().unwrap().delta, 10);
        assert_eq!(tracker.worst_moment().unwrap().delta, -15);
    }
}
