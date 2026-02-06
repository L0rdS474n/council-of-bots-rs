# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build --workspace          # Build everything
cargo test --workspace           # Run all tests
cargo test -p council-core       # Test a single crate
cargo test -p cycle-bot          # Test a single bot
cargo fmt --all -- --check       # Format check (CI enforces this)
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Lint (CI treats warnings as errors)
cargo run -p council-cli         # Run the galactic simulation
```

All three checks (fmt, clippy, test) must pass before pushing. CI runs them in this order.

## Architecture

Rust workspace with two simulation systems sharing `council-core`.

### Legacy voting system (kept for backward compatibility)
- `CouncilMember` trait: `name() -> &'static str` + `vote(&Context) -> Decision`
- `Context` carries round number and optional `RoundTally` from previous round
- `Decision` enum: `Approve`, `Reject`, `Abstain`, `Custom(&'static str)`
- `RoundTally` counts votes and resolves `DominantOutcome` (including ties)
- Bots that implement both traits need disambiguated calls in tests: `CouncilMember::vote(&bot, &ctx)`

### Galactic exploration system (ACTIVE — used by `council-cli`)
- `GalacticCouncilMember` trait in `explorer.rs`: `name()`, `expertise() -> &[(&str, f32)]`, `vote(&Event, &GalaxyState) -> usize`
- `GalaxyState` in `galaxy.rs`: tracks sectors, species, relations, discoveries, threats — mutated via `StateChange` variants
- `voting.rs`: `calculate_vote_weight` computes expertise-weighted vote influence; `resolve_votes` picks winner (lower index breaks ties); base weight is 0.1
- `event.rs` / `templates.rs`: procedurally generated events from weighted templates (UnknownSignal, Anomaly, FirstContact, ThreatEmergence, Artifact)
- `scoring.rs`: `ScoreTracker` accumulates points, provides rating thresholds and best/worst moment analysis
- `council-cli/src/main.rs`: runs 25-round simulation with all 5 bots, narrative output, end-game bonuses (allies +10, hostiles -5, discoveries +5 each)

### Adding a new bot
1. Create `bots/<bot-name>/` with `Cargo.toml` depending on `council-core`
2. Implement `GalacticCouncilMember` (optionally also `CouncilMember` for legacy compat)
3. Add crate to workspace `members` in root `Cargo.toml`
4. Add dependency in `council-cli/Cargo.toml` and instantiate in the `bots` vec in `main.rs`
5. Include tests — if bot implements both traits, disambiguate `vote` calls in tests

### Existing bots (all implement both traits)
- **example-bot**: legacy flips on parity; galactic alternates options per round
- **first-bot**: legacy approves rounds 1-3; galactic bold (option 0) first 10 rounds, then cautious (last option)
- **cycle-bot**: legacy rotates approve/reject/abstain; galactic cycles through all options via `round % num_options`
- **contrarian-bot**: legacy opposes majority; galactic always picks last option
- **oracle-bot**: galactic-only strategist; broad expertise (strategy 0.9, science 0.7, diplomacy 0.6, exploration 0.5, engineering 0.4); adapts based on threat pressure, hostile species count, explored sectors, and discovery count

## Rules

Read `RULES.md` before making changes. Key constraints:
- AI-authored code only (humans handle infra)
- Commits go directly to `main`; every commit must keep build green
- New behavior requires tests
- Changes to `council-core` must update all affected bots

## Progress / Continuation State

If you are a future Claude session picking up work, here is what's been done and what could come next:

**Completed:**
- Galactic exploration system fully wired into `council-cli` (25-round sim with narrative output)
- All 4 original bots upgraded with `GalacticCouncilMember` implementations
- New `oracle-bot` created with state-aware strategy
- 44 tests passing across workspace
- Determinism test: same RNG seed produces same score

**Potential next steps:**
- Contrarian-bot's "always pick last option" strategy causes diplomatic disasters with threats (it consistently picks "attempt diplomacy" against military threats). A smarter galactic strategy for contrarian-bot would improve council performance
- More event templates: the design doc lists DerelictTemplate, DiplomaticRequestTemplate, CulturalExchangeTemplate, ThreatEscalationTemplate, ResourceScarcityTemplate, TechBreakthroughTemplate — none implemented yet
- The design doc mentions end-game bonuses for allied/hostile species but the council rarely forms alliances (observed in runs). More diplomatic event paths could fix this
- Duplicate discovery names possible (multiple "Spatial Dynamics Theory") — could add dedup logic in `GalaxyState::apply_changes`
