# Galactic Council Exploration Simulation

**Date:** 2026-01-23
**Status:** Design approved

## Overview

Evolve the Council of Bots from a simple one-shot vote into a 25-round galactic exploration simulation. The council faces procedurally generated events (alien signals, anomalies, first contact, crises) and must vote on responses. Votes are weighted by bot expertise. Outcomes modify galaxy state and accumulate points toward a final score.

## Core Design Decisions

- **Procedural events** - Generated from templates based on current galaxy state
- **Role-based voting** - Bots declare expertise tags; relevant experts get higher vote weight
- **Flexible expertise** - Bots define their own tags (not a fixed set)
- **Multi-tag matching** - Events declare multiple weighted tags; partial matches contribute
- **Scoring system** - Points accumulate, final rating after 25 rounds (no hard game-over)

## Architecture

```
council-core/
├── lib.rs           # Re-exports
├── bot.rs           # CouncilMember trait (expanded)
├── event.rs         # Event struct, EventTemplate, response options
├── galaxy.rs        # GalaxyState (discoveries, threats, relations)
├── voting.rs        # Weighted voting logic
└── scoring.rs       # Score tracking

council-cli/
└── main.rs          # Run loop: generate event → collect votes → resolve → update state
```

## Expanded Bot Interface

```rust
pub trait CouncilMember {
    /// Bot's display name
    fn name(&self) -> &'static str;

    /// Expertise tags with proficiency levels (0.0 to 1.0)
    /// Example: [("diplomacy", 0.8), ("xenobiology", 0.6)]
    fn expertise(&self) -> &[(&'static str, f32)];

    /// Vote on an event given current galaxy state
    /// Returns index of chosen response option
    fn vote(&self, event: &Event, galaxy: &GalaxyState) -> usize;
}
```

### Expertise Matching

When an event has tags `[("diplomacy", 0.5), ("science", 0.3), ("military", 0.2)]` and a bot has `[("diplomacy", 0.8), ("engineering", 0.9)]`:

- Overlap: diplomacy only
- Bot's vote weight = `0.5 * 0.8 = 0.4` (event weight × bot proficiency)
- Bots with no matching expertise vote with baseline weight of `0.1`

## Event System

### Event Structure

```rust
pub struct Event {
    pub description: String,
    pub relevant_expertise: Vec<(String, f32)>,
    pub options: Vec<ResponseOption>,
}

pub struct ResponseOption {
    pub description: String,
    pub outcome: Outcome,
}

pub struct Outcome {
    pub description: String,
    pub score_delta: i32,
    pub state_changes: Vec<StateChange>,
}
```

### Event Templates

Templates generate events based on galaxy state:

```rust
pub trait EventTemplate {
    fn is_applicable(&self, galaxy: &GalaxyState) -> bool;
    fn generate(&self, galaxy: &GalaxyState, rng: &mut impl Rng) -> Event;
}
```

### Starter Templates

**Exploration:**
- `UnknownSignalTemplate` - Detect signal from unexplored region
- `AnomalyTemplate` - Spatial anomaly detected
- `DerelictTemplate` - Abandoned vessel found

**Contact:**
- `FirstContactTemplate` - New species encountered
- `DiplomaticRequestTemplate` - Known species makes request
- `CulturalExchangeTemplate` - Opportunity to share/learn

**Crisis:**
- `ThreatEmergenceTemplate` - New danger appears
- `ThreatEscalationTemplate` - Existing threat worsens
- `ResourceScarcityTemplate` - Critical shortage looms

**Discovery:**
- `TechBreakthroughTemplate` - Research yields results
- `ArtifactTemplate` - Ancient artifact found

## Galaxy State

```rust
pub struct GalaxyState {
    pub round: u32,
    pub score: i32,
    pub explored_sectors: Vec<Sector>,
    pub known_species: Vec<Species>,
    pub relations: HashMap<String, Relation>,
    pub discoveries: Vec<Discovery>,
    pub threats: Vec<Threat>,
}

pub struct Sector {
    pub name: String,
    pub sector_type: SectorType,
}

pub struct Species {
    pub name: String,
    pub traits: Vec<String>,
}

pub enum Relation {
    Unknown, Hostile, Wary, Neutral, Friendly, Allied,
}

pub struct Threat {
    pub name: String,
    pub severity: u32,
    pub rounds_active: u32,
}

pub enum StateChange {
    AddSector(Sector),
    AddSpecies(Species),
    SetRelation { species: String, relation: Relation },
    AddDiscovery(Discovery),
    AddThreat(Threat),
    RemoveThreat(String),
    ModifyThreatSeverity { name: String, delta: i32 },
}
```

## Voting Resolution

```rust
pub fn calculate_vote_weight(bot: &dyn CouncilMember, event: &Event) -> f32 {
    let base_weight = 0.1;

    let expertise_bonus: f32 = event.relevant_expertise
        .iter()
        .filter_map(|(tag, event_weight)| {
            bot.expertise()
                .iter()
                .find(|(bot_tag, _)| bot_tag == tag)
                .map(|(_, proficiency)| event_weight * proficiency)
        })
        .sum();

    base_weight + expertise_bonus
}

pub fn resolve_votes(votes: &[Vote], num_options: usize) -> usize {
    let mut totals = vec![0.0_f32; num_options];
    for vote in votes {
        totals[vote.chosen_option] += vote.weight;
    }
    totals.iter().enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}
```

## Scoring

| Outcome Type | Typical Points |
|--------------|----------------|
| Excellent response | +15 to +25 |
| Good response | +5 to +15 |
| Neutral/safe response | 0 to +5 |
| Poor response | -5 to -15 |
| Disastrous response | -15 to -30 |
| Unresolved threat (per round) | -3 per severity |
| Allied species (end bonus) | +10 each |
| Hostile species (end penalty) | -5 each |
| Discoveries (end bonus) | +5 each |

**Rating thresholds (25 rounds):**
- 200+: Legendary Council
- 150-199: Distinguished
- 100-149: Competent
- 50-99: Struggling
- Below 50: Dysfunctional

## Simulation Loop

```rust
fn main() {
    let rounds = 25;
    let mut galaxy = GalaxyState::new();
    let mut score = ScoreTracker::new();
    let bots = collect_all_bots();
    let templates = load_templates();

    for round in 1..=rounds {
        galaxy.round = round;

        let event = generate_event(&templates, &galaxy, &mut rng);
        let votes = collect_votes(&bots, &event, &galaxy);
        let winner = resolve_votes(&votes, event.options.len());
        let outcome = &event.options[winner].outcome;

        score.add(round, outcome.score_delta, &outcome.description);
        galaxy.apply_changes(&outcome.state_changes);

        let threat_penalty = galaxy.process_threats();
        if threat_penalty != 0 {
            score.add(round, threat_penalty, "Unresolved threats");
        }
    }

    print_final_report(&galaxy, &score);
}
```

## Testing Strategy

**Unit tests:**
- `voting.rs`: weight calculation, resolution, tie-breaking
- `galaxy.rs`: state change application, threat processing
- `event.rs`: template applicability

**Integration tests:**
- Full 25-round simulation with seeded RNG
- Determinism: same seed + same bots = same outcome

**Bot tests:**
- `expertise()` returns valid tags and weights (0.0-1.0)
- `vote()` returns valid option index

## Migration Path

1. **Expand council-core** - Add new modules, keep old trait working
2. **Update example-bot** - Implement new trait with expertise
3. **Upgrade council-cli** - Full simulation loop
4. **Remove deprecated code** - Clean up old trait

Each phase maintains green build per RULES.md.

## What's Explicitly Out of Scope

- Direct bot-to-bot communication
- Bot alliances or coalitions
- Bot elimination
- Persistent bot state between runs
- Network/async operations
