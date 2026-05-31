# lau-provenance

Every decision leaves a trail. Provenance records what was decided, why, what alternatives were considered, and what happened next. A decision diary that never forgets.

## The concept in 60 seconds

In the PLATO ecosystem, every agent action has provenance — a record of:

- **What was decided** (the action taken)
- **Why** (the reasoning, inputs, and state at decision time)
- **Alternatives** (what else was considered, and why it was rejected)
- **Outcome** (what happened after — was it a good decision?)
- **Pre-commit hook** (record provenance BEFORE executing, so failed actions are also captured)

This crate stores provenance in SQLite, queryable by time, agent, action type, or outcome quality.

## Quick start

```rust
use lau_provenance::{ProvenanceStore, Decision, Alternative};

let mut store = ProvenanceStore::in_memory();

// Record a decision with alternatives
let decision = Decision::new("deploy_service")
    .with_reasoning("Load exceeded threshold, scaling up")
    .with_input("cpu_percent", "87")
    .add_alternative(Alternative::new("do_nothing").rejected_because("would cause outage"))
    .add_alternative(Alternative::new("scale_down").rejected_because("wrong direction"));

store.record(decision);

// Query: what decisions were made in the last hour?
let recent = store.query().last_hour().by_action("deploy").execute();

// Decision quality audit
let quality = store.audit_quality();
println!("Good decisions: {}, Bad: {}", quality.good, quality.bad);
```

## Contributing

[Open an issue](https://github.com/SuperInstance/lau-provenance/issues) or PR.
