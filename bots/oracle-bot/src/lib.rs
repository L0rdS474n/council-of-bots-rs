use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;

/// OracleBot is a galactic strategist that analyzes the full state of the galaxy
/// to make informed decisions. It shifts priorities based on threats, diplomacy,
/// exploration progress, and discovery count.
///
/// Strategy:
/// - If active threats exist with high severity → prefer aggressive/military options (index 0)
/// - If hostile species outnumber allies → prefer diplomatic options (often index 0 or 1)
/// - If few sectors explored → prefer exploration/bold options (index 0)
/// - If galaxy is stable → prefer cautious/research options (index 1)
/// - Fallback: middle option as balanced choice
pub struct OracleBot;

impl GalacticCouncilMember for OracleBot {
    fn name(&self) -> &'static str {
        "oracle-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[
            ("strategy", 0.9),
            ("science", 0.7),
            ("diplomacy", 0.6),
            ("exploration", 0.5),
            ("engineering", 0.4),
        ]
    }

    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        let num_options = event.options.len();
        if num_options == 0 {
            return 0;
        }

        let threat_pressure = galaxy.threats.iter().map(|t| t.severity).sum::<u32>();
        let hostile_count = galaxy.hostile_count();
        let allied_count = galaxy.allied_count();
        let sectors_explored = galaxy.explored_sectors.len();
        let discovery_count = galaxy.discoveries.len();

        let is_threat_event = event
            .relevant_expertise
            .iter()
            .any(|(tag, _)| tag == "military" || tag == "strategy");
        let is_diplomacy_event = event
            .relevant_expertise
            .iter()
            .any(|(tag, _)| tag == "diplomacy" || tag == "culture" || tag == "linguistics");
        let is_exploration_event = event
            .relevant_expertise
            .iter()
            .any(|(tag, _)| tag == "exploration" || tag == "science");

        // High threat pressure: act decisively (bold option)
        if is_threat_event && threat_pressure >= 3 {
            return 0;
        }

        // Diplomatic crisis: hostile species dominate
        if is_diplomacy_event && hostile_count > allied_count {
            return 0; // Attempt peaceful contact
        }

        // Early game: explore aggressively
        if is_exploration_event && sectors_explored < 4 {
            return 0; // Bold exploration
        }

        // Mid-game stability: research and caution
        if discovery_count >= 3 && threat_pressure == 0 {
            return cautious_option(num_options);
        }

        // Default: balanced middle option
        balanced_option(num_options)
    }
}

/// Pick the cautious/research option (typically index 1).
fn cautious_option(num_options: usize) -> usize {
    if num_options >= 2 {
        1
    } else {
        0
    }
}

/// Pick a balanced middle option.
fn balanced_option(num_options: usize) -> usize {
    match num_options {
        0 | 1 => 0,
        2 => 1,
        _ => 1, // Middle option in 3-choice events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::event::{Outcome, ResponseOption};
    use council_core::galaxy::{GalaxyState, Relation, Species, Threat};

    fn make_event(expertise_tags: &[(&str, f32)], num_options: usize) -> Event {
        let options = (0..num_options)
            .map(|i| ResponseOption {
                description: format!("Option {}", i),
                outcome: Outcome {
                    description: format!("Outcome {}", i),
                    score_delta: 0,
                    state_changes: vec![],
                },
            })
            .collect();
        Event {
            description: "Test event".to_string(),
            relevant_expertise: expertise_tags
                .iter()
                .map(|(s, w)| (s.to_string(), *w))
                .collect(),
            options,
        }
    }

    #[test]
    fn oracle_has_broad_expertise() {
        let bot = OracleBot;
        let expertise = bot.expertise();
        assert!(expertise.len() >= 4);
        // Strategy should be highest
        assert_eq!(expertise[0], ("strategy", 0.9));
    }

    #[test]
    fn oracle_acts_boldly_under_threat() {
        let bot = OracleBot;
        let mut galaxy = GalaxyState::new();
        galaxy.threats.push(Threat {
            name: "Space Pirates".to_string(),
            severity: 4,
            rounds_active: 1,
        });
        let event = make_event(&[("military", 0.5), ("strategy", 0.3)], 3);
        assert_eq!(bot.vote(&event, &galaxy), 0);
    }

    #[test]
    fn oracle_explores_early() {
        let bot = OracleBot;
        let galaxy = GalaxyState::new(); // Only Home Sector
        let event = make_event(&[("exploration", 0.4), ("science", 0.3)], 3);
        assert_eq!(bot.vote(&event, &galaxy), 0);
    }

    #[test]
    fn oracle_diplomacy_when_hostile() {
        let bot = OracleBot;
        let mut galaxy = GalaxyState::new();
        galaxy.known_species.push(Species {
            name: "Zorblax".to_string(),
            traits: vec!["aggressive".to_string()],
        });
        galaxy
            .relations
            .insert("Zorblax".to_string(), Relation::Hostile);
        let event = make_event(&[("diplomacy", 0.5), ("culture", 0.3)], 3);
        assert_eq!(bot.vote(&event, &galaxy), 0);
    }

    #[test]
    fn oracle_cautious_when_stable() {
        let bot = OracleBot;
        let mut galaxy = GalaxyState::new();
        // Add enough discoveries to trigger cautious mode
        for i in 0..4 {
            galaxy.discoveries.push(council_core::galaxy::Discovery {
                name: format!("Discovery {}", i),
                category: "science".to_string(),
            });
        }
        let event = make_event(&[("archaeology", 0.4)], 3);
        assert_eq!(bot.vote(&event, &galaxy), 1);
    }

    #[test]
    fn oracle_returns_valid_index() {
        let bot = OracleBot;
        let galaxy = GalaxyState::new();
        for n in 1..=5 {
            let event = make_event(&[("science", 0.5)], n);
            let choice = bot.vote(&event, &galaxy);
            assert!(
                choice < n,
                "choice {} out of bounds for {} options",
                choice,
                n
            );
        }
    }
}
