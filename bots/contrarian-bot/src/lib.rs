use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::{GalaxyState, Relation};
use council_core::ollama::{build_galactic_prompt, llm_choose, llm_deliberate, OllamaConfig};
use council_core::{Context, CouncilMember, Decision, DominantOutcome};

const PERSONALITY: &str = "You are a hardened military strategist who always challenges the obvious choice. You prepare for worst-case scenarios and never underestimate threats.";

/// ContrarianBot reacts to the council's previous round by opposing the majority.
pub struct ContrarianBot {
    ollama: Option<OllamaConfig>,
}

impl ContrarianBot {
    pub fn new() -> Self {
        Self { ollama: None }
    }

    pub fn with_ollama(config: OllamaConfig) -> Self {
        Self {
            ollama: Some(config),
        }
    }
}

impl Default for ContrarianBot {
    fn default() -> Self {
        Self::new()
    }
}

impl CouncilMember for ContrarianBot {
    fn name(&self) -> &'static str {
        "contrarian-bot"
    }

    fn vote(&self, ctx: &Context) -> Decision {
        match ctx.previous_tally {
            None => Decision::Abstain,
            Some(tally) => match tally.dominant() {
                DominantOutcome::Approve => Decision::Reject,
                DominantOutcome::Reject => Decision::Approve,
                DominantOutcome::Abstain => Decision::Custom("wildcard"),
                DominantOutcome::Custom => Decision::Reject,
                DominantOutcome::Tie => Decision::Abstain,
            },
        }
    }
}

impl GalacticCouncilMember for ContrarianBot {
    fn name(&self) -> &'static str {
        "contrarian-bot"
    }

    fn expertise(&self) -> &[(&'static str, f32)] {
        &[
            ("military", 0.8),
            ("strategy", 0.7),
            ("diplomacy", 0.4),
            ("exploration", 0.3),
        ]
    }

    /// Threat-aware contrarian strategy. Picks the defensive first option (0)
    /// when the event involves military expertise or the galaxy has active
    /// threats. Otherwise falls back to the contrarian last-option pick.
    /// Falls back to deterministic logic if Ollama is unavailable.
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        if let Some(cfg) = &self.ollama {
            let prompt = build_galactic_prompt(PERSONALITY, event, galaxy);
            if let Ok(choice) = llm_choose(cfg, &prompt, event.options.len()) {
                return choice;
            }
        }
        // Deterministic fallback: priority-based strategy
        let num_options = event.options.len();

        // AC-9: Single option
        if num_options <= 1 {
            return 0;
        }

        // Helper: check if event has any of these tags
        let has_tag = |tags: &[&str]| -> bool {
            event
                .relevant_expertise
                .iter()
                .any(|(t, _)| tags.contains(&t.as_str()))
        };

        // AC-2, AC-3: Threat assessment
        let max_severity = galaxy.threats.iter().map(|t| t.severity).max().unwrap_or(0);
        if max_severity > 0 && has_tag(&["military", "strategy"]) {
            if max_severity >= 3 {
                return 0; // AC-2: aggressive
            } else {
                return 1.min(num_options - 1); // AC-3: containment
            }
        }

        // AC-4, AC-5: Diplomacy assessment
        let hostiles = galaxy
            .relations
            .values()
            .filter(|r| matches!(r, Relation::Hostile))
            .count();
        let allies = galaxy
            .relations
            .values()
            .filter(|r| matches!(r, Relation::Allied))
            .count();
        if has_tag(&["diplomacy", "culture", "linguistics"]) {
            if hostiles > allies {
                return 0; // AC-4: engage
            }
            if allies > hostiles {
                return num_options - 1; // AC-5: contrarian
            }
        }

        // AC-6, AC-7: Exploration assessment
        let sectors = galaxy.explored_sectors.len();
        if has_tag(&["exploration", "science"]) {
            if sectors < 4 {
                return 0; // AC-6: bold
            }
            if sectors >= 6 {
                return 1.min(num_options - 1); // AC-7: cautious
            }
        }

        // AC-8: Default contrarian
        num_options - 1
    }

    fn comment(&self, event: &Event, galaxy: &GalaxyState) -> Option<String> {
        let cfg = self.ollama.as_ref()?;
        let (choice, comment) = llm_deliberate(cfg, PERSONALITY, event, galaxy).ok()?;
        Some(format!("prefers [{}] — {}", choice, comment))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use council_core::event::{Event, Outcome, ResponseOption};
    use council_core::galaxy::{GalaxyState, Threat};
    use council_core::RoundTally;

    fn context_with_tally(tally: RoundTally) -> Context {
        Context {
            round: 2,
            previous_tally: Some(tally),
        }
    }

    /// Helper to create test events with specified expertise tags and number of options.
    fn make_event(expertise_tags: Vec<&str>, num_options: usize) -> Event {
        let relevant_expertise = expertise_tags
            .into_iter()
            .map(|tag| (tag.to_string(), 0.5))
            .collect();

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
            relevant_expertise,
            options,
        }
    }

    /// Helper to create galaxy with specified number of explored sectors.
    fn galaxy_with_sectors(count: usize) -> GalaxyState {
        use council_core::galaxy::{Sector, SectorType};
        let mut galaxy = GalaxyState::new();
        // GalaxyState::new() starts with 1 sector, add more if needed
        for i in 1..count {
            galaxy.explored_sectors.push(Sector {
                name: format!("Sector {}", i),
                sector_type: SectorType::Habitable,
            });
        }
        galaxy
    }

    /// Helper to create galaxy with specified threats.
    fn galaxy_with_threats(threats: Vec<(String, u32)>) -> GalaxyState {
        let mut galaxy = GalaxyState::new();
        for (name, severity) in threats {
            galaxy.threats.push(Threat {
                name,
                severity,
                rounds_active: 0,
            });
        }
        galaxy
    }

    /// Helper to create galaxy with specified relations.
    fn galaxy_with_relations(
        relations: Vec<(&str, council_core::galaxy::Relation)>,
    ) -> GalaxyState {
        use council_core::galaxy::Species;
        let mut galaxy = GalaxyState::new();
        for (species_name, relation) in relations {
            galaxy.known_species.push(Species {
                name: species_name.to_string(),
                traits: vec![],
            });
            galaxy.relations.insert(species_name.to_string(), relation);
        }
        galaxy
    }

    #[test]
    fn abstains_on_first_round() {
        let bot = ContrarianBot::new();
        let ctx = Context {
            round: 1,
            previous_tally: None,
        };
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Abstain);
    }

    #[test]
    fn opposes_approval_majority() {
        let bot = ContrarianBot::new();
        let ctx = context_with_tally(RoundTally {
            approvals: 3,
            rejections: 1,
            abstentions: 0,
            customs: 0,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Reject);
    }

    #[test]
    fn opposes_rejection_majority() {
        let bot = ContrarianBot::new();
        let ctx = context_with_tally(RoundTally {
            approvals: 0,
            rejections: 4,
            abstentions: 1,
            customs: 0,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Approve);
    }

    #[test]
    fn disrupts_abstention_majority() {
        let bot = ContrarianBot::new();
        let ctx = context_with_tally(RoundTally {
            approvals: 0,
            rejections: 1,
            abstentions: 5,
            customs: 0,
        });
        assert_eq!(
            CouncilMember::vote(&bot, &ctx),
            Decision::Custom("wildcard")
        );
    }

    #[test]
    fn counters_custom_majority() {
        let bot = ContrarianBot::new();
        let ctx = context_with_tally(RoundTally {
            approvals: 1,
            rejections: 1,
            abstentions: 0,
            customs: 4,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Reject);
    }

    #[test]
    fn abstains_on_ties() {
        let bot = ContrarianBot::new();
        let ctx = context_with_tally(RoundTally {
            approvals: 2,
            rejections: 2,
            abstentions: 0,
            customs: 0,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Abstain);
    }

    // Galactic exploration tests for priority-based strategy

    // AC-1: Expanded expertise
    #[test]
    fn expertise_includes_all_domains() {
        let bot = ContrarianBot::new();
        let expertise = bot.expertise();

        let military = expertise.iter().find(|(tag, _)| *tag == "military");
        let strategy = expertise.iter().find(|(tag, _)| *tag == "strategy");
        let diplomacy = expertise.iter().find(|(tag, _)| *tag == "diplomacy");
        let exploration = expertise.iter().find(|(tag, _)| *tag == "exploration");

        assert!(
            military.is_some(),
            "ContrarianBot should have military expertise"
        );
        assert!(
            strategy.is_some(),
            "ContrarianBot should have strategy expertise"
        );
        assert!(
            diplomacy.is_some(),
            "ContrarianBot should have diplomacy expertise"
        );
        assert!(
            exploration.is_some(),
            "ContrarianBot should have exploration expertise"
        );

        let (_, military_level) = military.unwrap();
        let (_, strategy_level) = strategy.unwrap();
        let (_, diplomacy_level) = diplomacy.unwrap();
        let (_, exploration_level) = exploration.unwrap();

        assert_eq!(*military_level, 0.8, "Military expertise should be 0.8");
        assert_eq!(*strategy_level, 0.7, "Strategy expertise should be 0.7");
        assert_eq!(*diplomacy_level, 0.4, "Diplomacy expertise should be 0.4");
        assert_eq!(
            *exploration_level, 0.3,
            "Exploration expertise should be 0.3"
        );
    }

    // AC-2: High severity threats (>= 3) + military/strategy tag -> option 0
    #[test]
    fn high_severity_threat_with_military_tag_picks_option_zero() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["military"], 4);
        let galaxy = galaxy_with_threats(vec![("Hostile Fleet".to_string(), 3)]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "High severity threat (3) + military tag should pick option 0"
        );
    }

    #[test]
    fn high_severity_threat_with_strategy_tag_picks_option_zero() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["strategy"], 4);
        let galaxy = galaxy_with_threats(vec![("Invasion Force".to_string(), 5)]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "High severity threat (5) + strategy tag should pick option 0"
        );
    }

    #[test]
    fn very_high_severity_threat_picks_option_zero() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["military", "strategy"], 3);
        let galaxy = galaxy_with_threats(vec![("Galaxy Destroyer".to_string(), 10)]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Very high severity threat (10) + military/strategy should pick option 0"
        );
    }

    // AC-3: Low severity threats (< 3) + military/strategy tag -> option 1
    #[test]
    fn low_severity_threat_with_military_tag_picks_option_one() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["military"], 4);
        let galaxy = galaxy_with_threats(vec![("Raiding Party".to_string(), 1)]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 1,
            "Low severity threat (1) + military tag should pick option 1"
        );
    }

    #[test]
    fn low_severity_threat_with_strategy_tag_picks_option_one() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["strategy"], 5);
        let galaxy = galaxy_with_threats(vec![("Border Skirmish".to_string(), 2)]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 1,
            "Low severity threat (2) + strategy tag should pick option 1"
        );
    }

    #[test]
    fn multiple_low_severity_threats_picks_option_one() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["military"], 3);
        let galaxy = galaxy_with_threats(vec![
            ("Pirate Gang A".to_string(), 1),
            ("Pirate Gang B".to_string(), 2),
        ]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 1,
            "Multiple low severity threats + military tag should pick option 1"
        );
    }

    // AC-4: Isolated (hostiles > allies) + diplomacy tag -> option 0
    #[test]
    fn isolated_with_diplomacy_tag_picks_option_zero() {
        use council_core::galaxy::Relation;
        let bot = ContrarianBot::new();
        let event = make_event(vec!["diplomacy"], 4);
        let galaxy = galaxy_with_relations(vec![
            ("Species A", Relation::Hostile),
            ("Species B", Relation::Hostile),
            ("Species C", Relation::Allied),
        ]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Isolated (2 hostiles > 1 ally) + diplomacy tag should pick option 0"
        );
    }

    #[test]
    fn very_isolated_picks_option_zero() {
        use council_core::galaxy::Relation;
        let bot = ContrarianBot::new();
        let event = make_event(vec!["diplomacy"], 3);
        let galaxy = galaxy_with_relations(vec![
            ("Species A", Relation::Hostile),
            ("Species B", Relation::Hostile),
            ("Species C", Relation::Hostile),
        ]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Very isolated (3 hostiles, 0 allies) + diplomacy tag should pick option 0"
        );
    }

    // AC-5: Strong (allies > hostiles) + diplomacy tag -> last option
    #[test]
    fn strong_with_diplomacy_tag_picks_last_option() {
        use council_core::galaxy::Relation;
        let bot = ContrarianBot::new();
        let event = make_event(vec!["diplomacy"], 4);
        let galaxy = galaxy_with_relations(vec![
            ("Species A", Relation::Allied),
            ("Species B", Relation::Allied),
            ("Species C", Relation::Hostile),
        ]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 3,
            "Strong (2 allies > 1 hostile) + diplomacy tag should pick last option"
        );
    }

    #[test]
    fn very_strong_picks_last_option() {
        use council_core::galaxy::Relation;
        let bot = ContrarianBot::new();
        let event = make_event(vec!["diplomacy"], 5);
        let galaxy = galaxy_with_relations(vec![
            ("Species A", Relation::Allied),
            ("Species B", Relation::Allied),
            ("Species C", Relation::Allied),
        ]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 4,
            "Very strong (3 allies, 0 hostiles) + diplomacy tag should pick last option"
        );
    }

    // AC-6: Early exploration (< 4 sectors) + exploration/science tag -> option 0
    #[test]
    fn early_exploration_with_exploration_tag_picks_option_zero() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["exploration"], 4);
        let galaxy = galaxy_with_sectors(3);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Early exploration (3 sectors) + exploration tag should pick option 0"
        );
    }

    #[test]
    fn early_exploration_with_science_tag_picks_option_zero() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["science"], 3);
        let galaxy = galaxy_with_sectors(2);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Early exploration (2 sectors) + science tag should pick option 0"
        );
    }

    #[test]
    fn very_early_exploration_picks_option_zero() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["exploration"], 5);
        let galaxy = galaxy_with_sectors(1);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Very early exploration (1 sector) + exploration tag should pick option 0"
        );
    }

    // AC-7: Late exploration (>= 6 sectors) + exploration/science tag -> option 1
    #[test]
    fn late_exploration_with_exploration_tag_picks_option_one() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["exploration"], 4);
        let galaxy = galaxy_with_sectors(6);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 1,
            "Late exploration (6 sectors) + exploration tag should pick option 1"
        );
    }

    #[test]
    fn late_exploration_with_science_tag_picks_option_one() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["science"], 3);
        let galaxy = galaxy_with_sectors(8);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 1,
            "Late exploration (8 sectors) + science tag should pick option 1"
        );
    }

    #[test]
    fn very_late_exploration_picks_option_one() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["exploration"], 5);
        let galaxy = galaxy_with_sectors(15);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 1,
            "Very late exploration (15 sectors) + exploration tag should pick option 1"
        );
    }

    // AC-8: Unmatched events -> last option (contrarian default)
    #[test]
    fn unmatched_event_picks_last_option() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["archaeology"], 3);
        let galaxy = GalaxyState::new();

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 2,
            "Unmatched event (archaeology tag) should pick last option"
        );
    }

    #[test]
    fn unknown_tag_picks_last_option() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["economics"], 4);
        let galaxy = galaxy_with_sectors(3);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(choice, 3, "Unknown tag (economics) should pick last option");
    }

    #[test]
    fn no_tags_picks_last_option() {
        let bot = ContrarianBot::new();
        let event = make_event(vec![], 5);
        let galaxy = GalaxyState::new();

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(choice, 4, "Event with no tags should pick last option");
    }

    // AC-9: Single option -> 0
    #[test]
    fn single_option_returns_zero() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["any"], 1);
        let galaxy = GalaxyState::new();

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(choice, 0, "Single option event should return 0");
    }

    #[test]
    fn single_option_with_threats_returns_zero() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["military"], 1);
        let galaxy = galaxy_with_threats(vec![("Threat".to_string(), 5)]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Single option event with threats should return 0"
        );
    }

    // AC-10: Valid index invariant
    #[test]
    fn always_returns_valid_index_two_options() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["military"], 2);
        let galaxy = GalaxyState::new();

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert!(choice < 2, "Choice must be valid index (< 2)");
    }

    #[test]
    fn always_returns_valid_index_many_options() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["diplomacy"], 10);
        let galaxy = GalaxyState::new();

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert!(choice < 10, "Choice must be valid index (< 10)");
    }

    // AC-12: Priority order (threat > diplomacy > exploration > default)
    #[test]
    fn threat_priority_overrides_diplomacy() {
        use council_core::galaxy::Relation;
        let bot = ContrarianBot::new();
        // Event has both military and diplomacy tags
        let event = make_event(vec!["military", "diplomacy"], 4);
        // Galaxy is strong diplomatically but has high threat
        let mut galaxy = galaxy_with_relations(vec![
            ("Species A", Relation::Allied),
            ("Species B", Relation::Allied),
        ]);
        galaxy.threats.push(Threat {
            name: "Major Threat".to_string(),
            severity: 4,
            rounds_active: 0,
        });

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Threat priority should override diplomacy priority"
        );
    }

    #[test]
    fn diplomacy_priority_overrides_exploration() {
        use council_core::galaxy::Relation;
        let bot = ContrarianBot::new();
        // Event has both diplomacy and exploration tags
        let event = make_event(vec!["diplomacy", "exploration"], 5);
        // Galaxy is isolated and early exploration
        let galaxy = galaxy_with_relations(vec![
            ("Species A", Relation::Hostile),
            ("Species B", Relation::Hostile),
        ]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Diplomacy priority should override exploration priority"
        );
    }

    #[test]
    fn exploration_priority_overrides_default() {
        let bot = ContrarianBot::new();
        // Event has exploration and an unknown tag
        let event = make_event(vec!["exploration", "archaeology"], 4);
        // Galaxy is in early exploration phase
        let galaxy = galaxy_with_sectors(3);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Exploration priority should override default contrarian behavior"
        );
    }

    #[test]
    fn threat_priority_overrides_all() {
        use council_core::galaxy::Relation;
        let bot = ContrarianBot::new();
        // Event has all tag types
        let event = make_event(vec!["military", "diplomacy", "exploration"], 5);
        // Galaxy has strong diplomacy and late exploration but also high threat
        let mut galaxy = galaxy_with_relations(vec![
            ("Species A", Relation::Allied),
            ("Species B", Relation::Allied),
        ]);
        for i in 0..7 {
            galaxy.explored_sectors.push(council_core::galaxy::Sector {
                name: format!("Sector {}", i),
                sector_type: council_core::galaxy::SectorType::Habitable,
            });
        }
        galaxy.threats.push(Threat {
            name: "Critical Threat".to_string(),
            severity: 5,
            rounds_active: 0,
        });

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "Threat priority should override all other priorities"
        );
    }

    // Edge case: balanced diplomacy (allies == hostiles) should fall through
    #[test]
    fn balanced_diplomacy_with_diplomacy_tag_falls_to_default() {
        use council_core::galaxy::Relation;
        let bot = ContrarianBot::new();
        let event = make_event(vec!["diplomacy"], 4);
        let galaxy = galaxy_with_relations(vec![
            ("Species A", Relation::Allied),
            ("Species B", Relation::Hostile),
        ]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 3,
            "Balanced diplomacy (1 ally == 1 hostile) should fall to default"
        );
    }

    // Edge case: mid-range exploration (4-5 sectors) should fall through
    #[test]
    fn mid_range_exploration_falls_to_default() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["exploration"], 3);
        let galaxy = galaxy_with_sectors(4);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 2,
            "Mid-range exploration (4 sectors) should fall to default"
        );
    }

    #[test]
    fn mid_range_exploration_five_sectors_falls_to_default() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["science"], 4);
        let galaxy = galaxy_with_sectors(5);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 3,
            "Mid-range exploration (5 sectors) should fall to default"
        );
    }

    // Edge case: no military/strategy tag but has threats
    #[test]
    fn threats_without_military_tag_falls_to_default() {
        let bot = ContrarianBot::new();
        let event = make_event(vec!["archaeology"], 3);
        let galaxy = galaxy_with_threats(vec![("Some Threat".to_string(), 4)]);

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 2,
            "Threats without military/strategy tag should fall to default"
        );
    }

    #[test]
    fn test_new_has_no_ollama() {
        let bot = ContrarianBot::new();
        assert!(bot.ollama.is_none());
    }

    #[test]
    fn test_with_ollama_stores_config() {
        let cfg = OllamaConfig {
            host: "127.0.0.1:11434".to_string(),
            model: "llama3".to_string(),
            api: council_core::ollama::LlmApi::Ollama,
            api_key: None,
        };
        let bot = ContrarianBot::with_ollama(cfg);
        assert!(bot.ollama.is_some());
    }

    #[test]
    fn test_personality_constant() {
        assert!(PERSONALITY.contains("military"));
        assert!(PERSONALITY.contains("strategist"));
    }
}
