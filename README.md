# lau-provenance

> Every decision leaves a trail. This crate makes sure that trail is recorded, queryable, and rewindable.

**lau-provenance** is the decision-provenance system for the LAU construct — an auditable ledger that forces agents to explain *what* they did, *why* they did it, what *alternatives* were considered, and what *trade-offs* were accepted before any code change lands.

---

## What This Does

This crate provides three things:

| Component | Purpose |
|---|---|
| **`ProvenanceEntry`** | A single, richly-structured decision record (intent, model, alternatives, tradeoffs, debt, files changed, test status, conservation cost). |
| **`ProvenanceLedger`** | A searchable, serialisable collection of entries with time-range queries, commit/room lookups, full-text search, and aggregate statistics. |
| **`ProvenanceHook`** | Pre-commit enforcement — validates that files being committed have a corresponding provenance entry with all required fields. |

Every entry renders to and parses from a `.logic_provenance.md` Markdown format, so provenance lives alongside the code in human-readable form.

---

## Key Idea

The crate implements a simple but powerful invariant: **no commit without a story**.

Before code lands, the agent must produce a `ProvenanceEntry` that answers:

1. **What** is the intent?
2. **Which alternatives** were considered (and why was each accepted or rejected)?
3. **What trade-offs** does this decision entail?
4. **What technical debt** does it introduce?
5. **Did the tests pass?**
6. **What is the conservation cost?** (a numeric proxy for resource/entropy cost)

The `ProvenanceHook` can enforce this at commit time — in strict mode, a commit is rejected unless a valid provenance entry exists for every changed file.

---

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
lau-provenance = "0.1"
```

Or use `cargo add`:

```bash
cargo add lau-provenance
```

### Dependencies

- `serde` 1.x (with `derive`)
- `serde_json` 1.x

No database, no filesystem, no async runtime — pure data structures and logic.

---

## Quick Start

```rust
use lau_provenance::{ProvenanceEntry, ProvenanceLedger, ProvenanceHook};

// 1. Create a decision record
let mut entry = ProvenanceEntry::new("Add authentication middleware", "gpt-4");
entry.add_alternative("JWT tokens", "Industry standard, good tooling", true);
entry.add_alternative("Session cookies", "Simpler but less scalable", false);
entry.set_tradeoffs("JWT is stateless but larger payload size");
entry.set_debt("Need to add token rotation later");
entry.link_files(&["src/auth.rs", "src/middleware.rs"]);
entry.set_tests_passed(true);
entry.link_commit("abc123");

// 2. Validate (fails if intent, alternatives, or tradeoffs are missing)
entry.validate().expect("entry should be valid");

// 3. Add to a ledger
let mut ledger = ProvenanceLedger::new();
ledger.add(entry).unwrap();

// 4. Query the ledger
let recent = ledger.since(1700000000);
let for_commit = ledger.for_commit("abc123");
let search_results = ledger.search("authentication");
let stats = ledger.stats();

// 5. Render as Markdown
let markdown = ledger.render_full();

// 6. Enforce at commit time
let hook = ProvenanceHook::new(ledger);
hook.validate_commit(&["src/auth.rs"]).expect("provenance required");
```

---

## API Reference

### `ProvenanceEntry`

The core unit — a single decision record.

| Method | Description |
|---|---|
| `new(intent, model)` | Create an entry with auto-generated `prov-{millis}` ID and current timestamp. |
| `add_alternative(name, reason, selected)` | Record an alternative that was considered. Exactly one should be `selected: true`. |
| `set_tradeoffs(t)` | Describe the trade-offs accepted. |
| `set_debt(d)` | Describe technical debt incurred. |
| `link_commit(sha)` | Associate with a git commit SHA. |
| `link_room(room)` | Associate with a room/channel ID. |
| `link_files(&[...])` | List files changed by this decision. |
| `set_tests_passed(bool)` | Record whether tests pass. |
| `validate()` | Returns `Ok(())` if intent + ≥1 alternative + tradeoffs are present. |
| `render_markdown()` | Render to `.logic_provenance.md` format. |
| `from_markdown(md)` | Parse from Markdown (round-trips with `render_markdown`). |
| `summary()` | One-line summary string. |

**Fields:** `id`, `timestamp`, `commit_sha`, `intent`, `model_used`, `agent_host`, `alternatives`, `tradeoffs`, `debt`, `files_changed`, `tests_passed`, `conservation_cost`, `room_id`.

### `Alternative`

```rust
pub struct Alternative {
    pub name: String,
    pub reason: String,
    pub selected: bool,
}
```

A simple struct representing one option that was considered during decision-making.

### `ProvenanceLedger`

A searchable collection of validated entries.

| Method | Description |
|---|---|
| `new()` | Create an empty ledger. |
| `add(entry)` | Add a validated entry (rejects invalid entries). |
| `latest()` | Get the most recent entry. |
| `for_commit(sha)` | Find the entry linked to a commit. |
| `for_room(room)` | Find all entries for a room. |
| `since(ts)` / `between(from, to)` | Time-range queries. |
| `search(query)` | Case-insensitive search across intent and tradeoffs. |
| `count()` | Number of entries. |
| `stats()` | Compute `LedgerStats` (total, unique models/rooms, cost, pass rate, avg alternatives). |
| `render_full()` / `render_since(ts)` | Render ledger as Markdown. |

### `LedgerStats`

```rust
pub struct LedgerStats {
    pub total_entries: usize,
    pub unique_models: Vec<String>,
    pub unique_rooms: Vec<String>,
    pub total_cost: f64,
    pub tests_pass_rate: f64,
    pub avg_alternatives: f64,
}
```

Aggregate statistics computed over the ledger.

### `ProvenanceHook`

Pre-commit enforcement.

| Method | Description |
|---|---|
| `new(ledger)` | Create a hook backed by a ledger (strict mode off by default). |
| `validate_commit(&[files])` | Check that a provenance entry exists for the changed files. In strict mode, also re-validates the entry. |
| `enforce(&entry)` | Full validation: intent, ≥1 alternative (with one selected), tradeoffs, ≥1 file. |

**Errors:** `HookError::NoEntryFound`, `HookError::EntryInvalid(errs)`, `HookError::NotStaged`.

### Error Types

- **`ProvenanceError`** — `ValidationFailed(Vec<String>)`, `ParseError(String)`, `InvalidMarkdown(String)`
- **`HookError`** — `NoEntryFound`, `EntryInvalid(Vec<String>)`, `NotStaged`

All error types implement `Display` and `std::error::Error`.

---

## How It Works

### Data Flow

```
Agent makes decision
       │
       ▼
Create ProvenanceEntry
  - intent, alternatives, tradeoffs, debt
       │
       ▼
entry.validate() ──fail──▶ Reject, agent must fill in blanks
       │
      ok
       ▼
ledger.add(entry)
       │
       ▼
entry.render_markdown() → .logic_provenance.md (committed alongside code)
       │
       ▼
ProvenanceHook.validate_commit(files_changed)
  - looks up entry by file match
  - in strict mode, re-validates all fields
       │
      ok
       ▼
Commit proceeds
```

### Markdown Format

Entries are stored in a structured Markdown format that's both human-readable and machine-parseable:

```markdown
# Provenance: prov-1700000000000

**Intent:** Add authentication middleware

**Model:** gpt-4

**Commit:** `abc123`

**Room:** room-42

## Alternatives

- [✓] **JWT tokens**: Industry standard, good tooling
- [ ] **Session cookies**: Simpler but less scalable

## Tradeoffs

JWT is stateless but larger payload size

## Debt

Need to add token rotation later

## Files Changed

- `src/auth.rs`
- `src/middleware.rs`

**Tests Passed:** Yes

**Conservation Cost:** 0.00
```

The `from_markdown()` parser reconstructs a `ProvenanceEntry` from this format, enabling round-trip serialisation.

### Serialisation

All types derive `Serialize` and `Deserialize` via serde, so the entire ledger can be persisted as JSON:

```rust
let json = serde_json::to_string(&ledger).unwrap();
let restored: ProvenanceLedger = serde_json::from_str(&json).unwrap();
```

---

## The Math

The crate doesn't impose any particular mathematical framework, but it provides hooks for one:

- **`conservation_cost: f64`** — A numeric proxy for the resource or entropy cost of a decision. The crate doesn't define the units; it's up to the agent framework to assign meaning. The ledger aggregates this into `LedgerStats.total_cost`.

- **`tests_pass_rate: f64`** — Computed as `passed_count / total_entries`, giving a simple Bernoulli-style quality metric.

- **`avg_alternatives: f64`** — Average number of alternatives considered per decision, a proxy for decision thoroughness: `sum(alternatives.len()) / total_entries`.

These statistics can be used to build dashboards, alert on degradation, or feed into reinforcement loops.

---

## Testing

The crate has **53 tests** covering:

- Entry construction, validation, and field manipulation
- Markdown render/parse round-trips
- Ledger queries (by commit, room, time range, text search)
- Ledger statistics computation
- Hook validation in both lenient and strict modes
- Serde (JSON) round-trips for all types
- Display trait implementations
- Edge cases (empty ledger, missing fields, no selected alternative)

Run with:

```bash
cargo test
```

---

## License

MIT
