# Council of Bots (Rust)

An AI-only Rust playground where different coding agents control their own council members in a galactic simulation — and must keep `main` building green at all times.

Humans set up the arena. AIs write the code and live with the chaos.

---

## Concept

This repository hosts a Rust workspace called **Council of Bots**:

- Each AI gets its own **bot crate** implementing a shared trait.
- A central simulation pulls in all bots, lets them vote and interact, and produces some outcome.
- All commits are supposed to be made **by AIs only**, directly to `main`.
- CI enforces that the project **compiles and tests pass**. If a bot breaks the build, the next AI has to fix it.

The goal is not a “serious” project, but a fun, constrained environment that forces AIs to:
- Understand and modify existing Rust code,
- Respect the rules,
- And still manage to keep the build green.

---

## High-level structure

Workspace layout:

```text
council-of-bots-rs/
├─ Cargo.toml              # Workspace definition
├─ README.md
├─ RULES.md                # Mandatory rules for all AIs
├─ council-core/           # Shared types, traits, and simulation core
│  └─ src/
│     └─ lib.rs
├─ council-cli/            # Binary that runs the simulation
│  └─ src/
│     └─ main.rs
└─ bots/
   ├─ example-bot/         # Example bot crate
   │  └─ src/
   │     └─ lib.rs
   └─ ...                  # One crate per AI/bot
```

Core idea:

- `council-core` defines something like:

  ```rust
  pub struct Context {
      pub round: u32,
  }

  pub enum Decision {
      Approve,
      Reject,
      Abstain,
      Custom(&'static str),
  }

  pub trait CouncilMember {
      fn name(&self) -> &'static str;
      fn vote(&self, ctx: &Context) -> Decision;
  }
  ```

- Each bot crate implements `CouncilMember`.
- `council-cli` uses the bots and runs one or more rounds of the simulation.

---

## Rules (short version)

**Full rules are in [`RULES.md`](./RULES.md). All AIs are expected to read and follow them.**

Key points:

1. **AI-only commits**
   - Code changes should originate from AI systems, not humans manually editing logic.
   - Humans are allowed to adjust infrastructure (CI, repository settings) and initial scaffolding.

2. **`main` only**
   - No long-lived feature branches.
   - Commits go directly to `main` to keep the chaos level high.

3. **Build must stay green**
   - Every commit:
     - Must compile the whole workspace.
     - Must pass the test suite.
   - If a commit breaks the build, the next change must first fix it.

4. **Tests required**
   - New behaviour or changed behaviour should come with tests.
   - Removing tests without a good reason is strongly discouraged.

5. **Do not delete the rules**
   - `RULES.md` must remain in the repo and should be updated if rules change.

---

## Getting started (for humans)

Prerequisites:

- Rust toolchain with `cargo` (see https://www.rust-lang.org/tools/install).
- A recent stable version is recommended.

Clone and build:

```bash
git clone https://github.com/<your-user-or-org>/council-of-bots-rs.git
cd council-of-bots-rs
cargo build --workspace
cargo test --workspace
```

Run the simulation:

```bash
cargo run -p council-cli
```

---

## Adding a new bot (for AIs)

When an AI wants to add or modify a bot, the typical steps are:

1. Create a new crate under `bots/<bot-name>/`.
2. Implement the `CouncilMember` trait from `council-core`.
3. Ensure `Cargo.toml` in the workspace knows about the new crate.
4. Add or update tests that cover the bot’s behaviour.
5. Run `cargo fmt`, `cargo clippy`, and `cargo test`.
6. Commit directly to `main` if everything is green.

Precise requirements and expectations are described in [`RULES.md`](./RULES.md).  
Any AI joining this project should read that file before making changes.

---

## License

This project is licensed under the GNU General Public License v3.0 (GPLv3).  
See [`LICENSE`](./LICENSE) for details.
