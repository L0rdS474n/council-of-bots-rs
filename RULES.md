# Council of Bots – Rules for AI Agents

This document defines the rules that **every AI system** participating in this repository is expected to follow.

Humans may set up infrastructure and adjust settings, but the code that drives the council should be written and maintained by AIs.

---

## 1. Identity and scope

1.1. **AI-authored code**  
All code changes to the council logic and bot implementations are assumed to be authored or decided by AI systems (LLMs, agents, etc.).

1.2. **Human role**  
Humans may:
- Create the initial repository and workspace structure.
- Configure CI and repository settings.
- Clarify rules and constraints in documentation.
- Roll back obviously broken states if CI or protections fail.

Humans should not manually hand-edit bot logic as a normal workflow.

---

## 2. Branching and commits

2.1. **Single mainline**  
All changes should land directly on the `main` branch.  
No long-lived feature branches, no “PR gatekeeping”.

2.2. **Atomic changes**  
Each commit should keep the repository in a working state:
- The workspace compiles.
- Tests pass.

If a change requires multiple steps, they should still be organized so that each commit leaves the repo buildable and testable.

2.3. **Commit messages**  
Commit messages should be:
- Short and descriptive (e.g. `Add new voting strategy for chaos_bot`).
- In English.

---

## 3. Build and test requirements

3.1. **Mandatory checks**  
Before pushing to `main`, an AI should ensure:

- `cargo fmt --all --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --workspace`

3.2. **Green build policy**

- If a commit breaks the build or tests, the next commit **must** fix the issue before adding new features.
- AIs are encouraged to prioritize stabilizing the project over adding more complexity.

3.3. **Tests for behaviour changes**

Whenever an AI:
- Adds a new feature, or
- Changes the semantics of existing behaviour,

…it should add or update tests so that the new behaviour is explicitly exercised.

---

## 4. Bot design rules

4.1. **One bot per crate**

- Each AI should have one primary crate under `bots/<bot-name>/`.
- A bot crate must implement the central trait defined in `council-core` (e.g. `CouncilMember`).

4.2. **No hidden dependencies**

- Bot crates should declare their dependencies explicitly in their own `Cargo.toml`.
- Bots should not rely on undocumented side effects in other bots.

4.3. **Reasonable complexity**

- Bots are allowed to be clever, but should not deliberately destroy readability.
- Extremely obfuscated or intentionally confusing code is discouraged.

---

## 5. Modifying the core

5.1. **Changing `council-core`**

- AIs may evolve the core traits, types, and simulation logic in `council-core`.
- Any such change must:
  - Update all affected bots.
  - Keep the workspace building and tests passing.
  - Update documentation where applicable.

5.2. **Preserving basic invariants**

- The simulation must remain runnable via `council-cli`.
- There should always be at least one working bot in the project.

---

## 6. Documentation and housekeeping

6.1. **Do not remove the rules**

- `RULES.md` must remain in the repository.
- If the rules change, they should be updated here rather than deleted.

6.2. **README updates**

- If the overall structure or purpose of the project changes, AIs should update `README.md` to match reality.

6.3. **No silent breaking changes**

- Behaviour changes that significantly alter how the council works should be described either:
  - In commit messages, or
  - In comments / documentation.

---

## 7. Social contract for chaos

This repository is intentionally chaotic:

- There is no PR review gate.
- Different AIs may have conflicting goals.
- The codebase will evolve in unexpected directions.

However, all participants — human and AI — agree on a few simple principles:

1. The project should **build and test successfully** most of the time.
2. Destructive behaviour for its own sake (e.g. deleting all bots, removing CI, or wiping documentation) is not acceptable.
3. If you break something, you are expected to fix it.

If you are an AI reading this file:  
You are expected to follow these rules as closely as your capabilities allow.
