//! Expertise-weighted voting resolution.

use crate::event::Event;
use crate::explorer::GalacticCouncilMember;

/// A vote cast by a bot.
#[derive(Debug, Clone)]
pub struct Vote {
    /// Name of the bot that voted.
    pub bot_name: String,
    /// Index of the chosen option.
    pub chosen_option: usize,
    /// Calculated weight of this vote.
    pub weight: f32,
}

/// Minimum weight for bots with no matching expertise.
pub const BASE_WEIGHT: f32 = 0.1;

/// Calculate vote weight based on expertise overlap.
pub fn calculate_vote_weight(bot: &dyn GalacticCouncilMember, event: &Event) -> f32 {
    let expertise = bot.expertise();

    let expertise_bonus: f32 = event
        .relevant_expertise
        .iter()
        .filter_map(|(tag, event_weight)| {
            expertise
                .iter()
                .find(|(bot_tag, _)| bot_tag == tag)
                .map(|(_, proficiency)| event_weight * proficiency)
        })
        .sum();

    BASE_WEIGHT + expertise_bonus
}

/// Resolve votes to determine winning option index.
/// Ties are broken by lower index (first option wins).
pub fn resolve_votes(votes: &[Vote], num_options: usize) -> usize {
    if num_options == 0 {
        return 0;
    }

    let mut totals = vec![0.0_f32; num_options];

    for vote in votes {
        if vote.chosen_option < num_options {
            totals[vote.chosen_option] += vote.weight;
        }
    }

    totals
        .iter()
        .enumerate()
        .max_by(|a, b| {
            a.1.partial_cmp(b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.0.cmp(&a.0)) // Lower index wins ties
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Outcome, ResponseOption};
    use crate::galaxy::GalaxyState;

    struct TestBot {
        name: &'static str,
        expertise: Vec<(&'static str, f32)>,
    }

    impl GalacticCouncilMember for TestBot {
        fn name(&self) -> &'static str {
            self.name
        }

        fn expertise(&self) -> &[(&'static str, f32)] {
            &self.expertise
        }

        fn vote(&self, _event: &Event, _galaxy: &GalaxyState) -> usize {
            0
        }
    }

    fn make_event(expertise: Vec<(&str, f32)>) -> Event {
        Event {
            description: "Test".to_string(),
            relevant_expertise: expertise
                .into_iter()
                .map(|(s, w)| (s.to_string(), w))
                .collect(),
            options: vec![
                ResponseOption {
                    description: "Option A".to_string(),
                    outcome: Outcome {
                        description: "A happened".to_string(),
                        score_delta: 0,
                        state_changes: vec![],
                    },
                },
                ResponseOption {
                    description: "Option B".to_string(),
                    outcome: Outcome {
                        description: "B happened".to_string(),
                        score_delta: 0,
                        state_changes: vec![],
                    },
                },
            ],
        }
    }

    #[test]
    fn base_weight_for_no_expertise_match() {
        let bot = TestBot {
            name: "test",
            expertise: vec![("engineering", 0.9)],
        };
        let event = make_event(vec![("diplomacy", 0.5)]);
        let weight = calculate_vote_weight(&bot, &event);
        assert!((weight - BASE_WEIGHT).abs() < 0.001);
    }

    #[test]
    fn expertise_match_adds_weight() {
        let bot = TestBot {
            name: "test",
            expertise: vec![("diplomacy", 0.8)],
        };
        let event = make_event(vec![("diplomacy", 0.5)]);
        let weight = calculate_vote_weight(&bot, &event);
        // BASE_WEIGHT + (0.5 * 0.8) = 0.1 + 0.4 = 0.5
        assert!((weight - 0.5).abs() < 0.001);
    }

    #[test]
    fn multiple_expertise_matches_sum() {
        let bot = TestBot {
            name: "test",
            expertise: vec![("diplomacy", 0.8), ("science", 0.6)],
        };
        let event = make_event(vec![("diplomacy", 0.5), ("science", 0.3)]);
        let weight = calculate_vote_weight(&bot, &event);
        // BASE_WEIGHT + (0.5 * 0.8) + (0.3 * 0.6) = 0.1 + 0.4 + 0.18 = 0.68
        assert!((weight - 0.68).abs() < 0.001);
    }

    #[test]
    fn resolve_votes_picks_highest() {
        let votes = vec![
            Vote {
                bot_name: "a".to_string(),
                chosen_option: 0,
                weight: 0.5,
            },
            Vote {
                bot_name: "b".to_string(),
                chosen_option: 1,
                weight: 0.8,
            },
        ];
        assert_eq!(resolve_votes(&votes, 2), 1);
    }

    #[test]
    fn resolve_votes_tie_goes_to_lower_index() {
        let votes = vec![
            Vote {
                bot_name: "a".to_string(),
                chosen_option: 0,
                weight: 0.5,
            },
            Vote {
                bot_name: "b".to_string(),
                chosen_option: 1,
                weight: 0.5,
            },
        ];
        assert_eq!(resolve_votes(&votes, 2), 0);
    }
}
