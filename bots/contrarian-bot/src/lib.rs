use council_core::event::Event;
use council_core::explorer::GalacticCouncilMember;
use council_core::galaxy::GalaxyState;
use council_core::{Context, CouncilMember, Decision, DominantOutcome};

/// ContrarianBot reacts to the council's previous round by opposing the majority.
pub struct ContrarianBot;

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
        &[("military", 0.8), ("strategy", 0.6)]
    }

    /// Threat-aware contrarian strategy. Picks the defensive first option (0)
    /// when the event involves military expertise or the galaxy has active
    /// threats. Otherwise falls back to the contrarian last-option pick.
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize {
        let is_threat = event
            .relevant_expertise
            .iter()
            .any(|(tag, _)| tag == "military")
            || !galaxy.threats.is_empty();

        if is_threat {
            0
        } else {
            event.options.len().saturating_sub(1)
        }
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

    #[test]
    fn abstains_on_first_round() {
        let bot = ContrarianBot;
        let ctx = Context {
            round: 1,
            previous_tally: None,
        };
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Abstain);
    }

    #[test]
    fn opposes_approval_majority() {
        let bot = ContrarianBot;
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
        let bot = ContrarianBot;
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
        let bot = ContrarianBot;
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
        let bot = ContrarianBot;
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
        let bot = ContrarianBot;
        let ctx = context_with_tally(RoundTally {
            approvals: 2,
            rejections: 2,
            abstentions: 0,
            customs: 0,
        });
        assert_eq!(CouncilMember::vote(&bot, &ctx), Decision::Abstain);
    }

    // Galactic exploration tests for threat-aware strategy

    #[test]
    fn picks_first_option_on_military_event() {
        let bot = ContrarianBot;
        let event = make_event(vec!["military", "strategy"], 3);
        let galaxy = GalaxyState::new();

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "ContrarianBot should pick first (defensive) option when event has military expertise"
        );
    }

    #[test]
    fn picks_last_option_on_non_threat_event() {
        let bot = ContrarianBot;
        let event = make_event(vec!["science", "exploration"], 3);
        let galaxy = GalaxyState::new(); // empty galaxy, no threats

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 2,
            "ContrarianBot should pick last option when no military tag and no active threats"
        );
    }

    #[test]
    fn picks_first_option_when_galaxy_has_threats() {
        let bot = ContrarianBot;
        let event = make_event(vec!["diplomacy"], 4);
        let mut galaxy = GalaxyState::new();
        galaxy.threats.push(Threat {
            name: "Hostile Fleet".to_string(),
            severity: 3,
            rounds_active: 2,
        });

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "ContrarianBot should pick defensive first option when galaxy has active threats"
        );
    }

    #[test]
    fn single_option_returns_zero() {
        let bot = ContrarianBot;
        let event = make_event(vec!["any"], 1);
        let galaxy = GalaxyState::new();

        let choice = GalacticCouncilMember::vote(&bot, &event, &galaxy);
        assert_eq!(
            choice, 0,
            "ContrarianBot should return 0 for single-option events"
        );
    }

    #[test]
    fn expertise_includes_military_and_strategy() {
        let bot = ContrarianBot;
        let expertise = bot.expertise();

        let military = expertise.iter().find(|(tag, _)| *tag == "military");
        let strategy = expertise.iter().find(|(tag, _)| *tag == "strategy");

        assert!(
            military.is_some(),
            "ContrarianBot should have military expertise"
        );
        assert!(
            strategy.is_some(),
            "ContrarianBot should have strategy expertise"
        );

        let (_, military_level) = military.unwrap();
        assert!(
            *military_level > 0.0,
            "Military expertise level should be positive"
        );
    }
}
