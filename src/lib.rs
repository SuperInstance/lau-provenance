//! # LAU Provenance
//!
//! THE provenance system for the LAU construct.
//! Every decision is documented, rewindable, and queryable.
//! Forces agents to explain their reasoning before code changes land.

use serde::{Deserialize, Serialize};

/// Errors that can occur during provenance operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProvenanceError {
    ValidationFailed(Vec<String>),
    ParseError(String),
    InvalidMarkdown(String),
}

impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProvenanceError::ValidationFailed(errs) => {
                write!(f, "Validation failed: {}", errs.join(", "))
            }
            ProvenanceError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ProvenanceError::InvalidMarkdown(msg) => write!(f, "Invalid markdown: {}", msg),
        }
    }
}

impl std::error::Error for ProvenanceError {}

/// Errors from the provenance hook (pre-commit enforcement).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookError {
    NoEntryFound,
    EntryInvalid(Vec<String>),
    NotStaged,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookError::NoEntryFound => write!(f, "No provenance entry found"),
            HookError::EntryInvalid(errs) => {
                write!(f, "Entry invalid: {}", errs.join(", "))
            }
            HookError::NotStaged => write!(f, "Files not staged"),
        }
    }
}

impl std::error::Error for HookError {}

/// An alternative that was considered during decision-making.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Alternative {
    pub name: String,
    pub reason: String,
    pub selected: bool,
}

impl Alternative {
    pub fn new(name: &str, reason: &str, selected: bool) -> Self {
        Self {
            name: name.to_string(),
            reason: reason.to_string(),
            selected,
        }
    }
}

/// A single provenance entry — a decision record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceEntry {
    pub id: String,
    pub timestamp: u64,
    pub commit_sha: Option<String>,
    pub intent: String,
    pub model_used: String,
    pub agent_host: String,
    pub alternatives: Vec<Alternative>,
    pub tradeoffs: String,
    pub debt: String,
    pub files_changed: Vec<String>,
    pub tests_passed: bool,
    pub conservation_cost: f64,
    pub room_id: Option<String>,
}

impl ProvenanceEntry {
    /// Create a new provenance entry with the given intent and model.
    pub fn new(intent: &str, model: &str) -> Self {
        Self {
            id: format!(
                "prov-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            commit_sha: None,
            intent: intent.to_string(),
            model_used: model.to_string(),
            agent_host: String::new(),
            alternatives: Vec::new(),
            tradeoffs: String::new(),
            debt: String::new(),
            files_changed: Vec::new(),
            tests_passed: false,
            conservation_cost: 0.0,
            room_id: None,
        }
    }

    /// Add an alternative that was considered.
    pub fn add_alternative(&mut self, name: &str, reason: &str, selected: bool) {
        self.alternatives
            .push(Alternative::new(name, reason, selected));
    }

    /// Set the tradeoffs description.
    pub fn set_tradeoffs(&mut self, t: &str) {
        self.tradeoffs = t.to_string();
    }

    /// Set the technical debt incurred.
    pub fn set_debt(&mut self, d: &str) {
        self.debt = d.to_string();
    }

    /// Link this entry to a git commit.
    pub fn link_commit(&mut self, sha: &str) {
        self.commit_sha = Some(sha.to_string());
    }

    /// Link this entry to a room.
    pub fn link_room(&mut self, room: &str) {
        self.room_id = Some(room.to_string());
    }

    /// Link files that were changed.
    pub fn link_files(&mut self, files: &[&str]) {
        self.files_changed = files.iter().map(|s| s.to_string()).collect();
    }

    /// Set whether tests passed.
    pub fn set_tests_passed(&mut self, passed: bool) {
        self.tests_passed = passed;
    }

    /// Render as markdown in the .logic_provenance.md format.
    pub fn render_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Provenance: {}\n\n", self.id));
        md.push_str(&format!("**Intent:** {}\n\n", self.intent));
        md.push_str(&format!("**Model:** {}\n\n", self.model_used));
        if !self.agent_host.is_empty() {
            md.push_str(&format!("**Agent Host:** {}\n\n", self.agent_host));
        }
        if let Some(sha) = &self.commit_sha {
            md.push_str(&format!("**Commit:** `{}`\n\n", sha));
        }
        if let Some(room) = &self.room_id {
            md.push_str(&format!("**Room:** {}\n\n", room));
        }

        md.push_str("## Alternatives\n\n");
        for alt in &self.alternatives {
            let marker = if alt.selected { "✓" } else { " " };
            md.push_str(&format!("- [{}] **{}**: {}\n", marker, alt.name, alt.reason));
        }
        md.push('\n');

        md.push_str(&format!("## Tradeoffs\n\n{}\n\n", self.tradeoffs));
        md.push_str(&format!("## Debt\n\n{}\n\n", self.debt));

        if !self.files_changed.is_empty() {
            md.push_str("## Files Changed\n\n");
            for f in &self.files_changed {
                md.push_str(&format!("- `{}`\n", f));
            }
            md.push('\n');
        }

        md.push_str(&format!(
            "**Tests Passed:** {}\n\n",
            if self.tests_passed { "Yes" } else { "No" }
        ));
        md.push_str(&format!("**Conservation Cost:** {:.2}\n", self.conservation_cost));

        md
    }

    /// Validate this entry. Must have intent, at least 1 alternative, and tradeoffs.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.intent.trim().is_empty() {
            errors.push("Intent is required".to_string());
        }
        if self.alternatives.is_empty() {
            errors.push("At least one alternative is required".to_string());
        }
        if self.tradeoffs.trim().is_empty() {
            errors.push("Tradeoffs are required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Parse a provenance entry from markdown.
    pub fn from_markdown(md: &str) -> Result<Self, ProvenanceError> {
        let mut entry = Self::new("", "unknown");

        for line in md.lines() {
            let line = line.trim();
            if line.starts_with("# Provenance:") {
                entry.id = line
                    .trim_start_matches("# Provenance:")
                    .trim()
                    .to_string();
            } else if line.starts_with("**Intent:**") {
                entry.intent = line
                    .trim_start_matches("**Intent:**")
                    .trim()
                    .to_string();
            } else if line.starts_with("**Model:**") {
                entry.model_used = line
                    .trim_start_matches("**Model:**")
                    .trim()
                    .to_string();
            } else if line.starts_with("**Commit:**") {
                let sha = line
                    .trim_start_matches("**Commit:**")
                    .trim()
                    .trim_matches('`');
                entry.commit_sha = Some(sha.to_string());
            } else if line.starts_with("**Room:**") {
                entry.room_id = Some(
                    line.trim_start_matches("**Room:**")
                        .trim()
                        .to_string(),
                );
            } else if line.starts_with("- [✓]") || line.starts_with("- [ ]") {
                let selected = line.starts_with("- [✓]");
                let rest = if selected {
                    line.trim_start_matches("- [✓]")
                } else {
                    line.trim_start_matches("- [ ]")
                };
                let rest = rest.trim();
                // Parse **name**: reason
                if let Some(colon_pos) = rest.find(": ") {
                    let name = rest[..colon_pos].trim().trim_matches('*').to_string();
                    let reason = rest[colon_pos + 2..].to_string();
                    entry.alternatives.push(Alternative {
                        name,
                        reason,
                        selected,
                    });
                }
            } else if line.starts_with("**Tests Passed:**") {
                entry.tests_passed = line.contains("Yes");
            } else if line.starts_with("**Conservation Cost:**") {
                let cost_str = line
                    .trim_start_matches("**Conservation Cost:**")
                    .trim();
                entry.conservation_cost = cost_str.parse::<f64>().unwrap_or(0.0);
            }
        }

        // Extract tradeoffs section
        if let Some(start) = md.find("## Tradeoffs\n\n") {
            let rest = &md[start + "## Tradeoffs\n\n".len()..];
            if let Some(end) = rest.find("\n## ") {
                entry.tradeoffs = rest[..end].trim().to_string();
            } else {
                entry.tradeoffs = rest.trim().to_string();
            }
        }

        // Extract debt section
        if let Some(start) = md.find("## Debt\n\n") {
            let rest = &md[start + "## Debt\n\n".len()..];
            if let Some(end) = rest.find("\n## ") {
                entry.debt = rest[..end].trim().to_string();
            } else {
                // Take until Files Changed or end
                if let Some(end) = rest.find("\n**") {
                    entry.debt = rest[..end].trim().to_string();
                } else {
                    entry.debt = rest.trim().to_string();
                }
            }
        }

        // Extract files changed
        if let Some(start) = md.find("## Files Changed\n\n") {
            let rest = &md[start + "## Files Changed\n\n".len()..];
            let files: Vec<String> = rest
                .lines()
                .take_while(|l| l.starts_with("- `"))
                .map(|l| {
                    l.trim_start_matches("- `")
                        .trim_end_matches('`')
                        .to_string()
                })
                .collect();
            entry.files_changed = files;
        }

        // Give it a timestamp from the id if possible
        if entry.id.starts_with("prov-") {
            if let Ok(millis) = entry.id[5..].parse::<u64>() {
                entry.timestamp = millis / 1000;
            }
        }

        Ok(entry)
    }

    /// One-line summary of this entry.
    pub fn summary(&self) -> String {
        let sha = self
            .commit_sha
            .as_deref()
            .unwrap_or("no-commit");
        format!(
            "[{}] {} (model={}, files={}, cost={:.2})",
            sha,
            self.intent,
            self.model_used,
            self.files_changed.len(),
            self.conservation_cost
        )
    }
}

/// Statistics about a provenance ledger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LedgerStats {
    pub total_entries: usize,
    pub unique_models: Vec<String>,
    pub unique_rooms: Vec<String>,
    pub total_cost: f64,
    pub tests_pass_rate: f64,
    pub avg_alternatives: f64,
}

/// A collection of provenance entries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceLedger {
    pub entries: Vec<ProvenanceEntry>,
    pub repo_path: Option<String>,
}

impl ProvenanceLedger {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            repo_path: None,
        }
    }

    /// Add a validated entry to the ledger.
    pub fn add(&mut self, entry: ProvenanceEntry) -> Result<(), ProvenanceError> {
        match entry.validate() {
            Ok(()) => {
                self.entries.push(entry);
                Ok(())
            }
            Err(errs) => Err(ProvenanceError::ValidationFailed(errs)),
        }
    }

    /// Get the latest entry.
    pub fn latest(&self) -> Option<&ProvenanceEntry> {
        self.entries.last()
    }

    /// Find entry for a given commit SHA.
    pub fn for_commit(&self, sha: &str) -> Option<&ProvenanceEntry> {
        self.entries
            .iter()
            .find(|e| e.commit_sha.as_deref() == Some(sha))
    }

    /// Find all entries for a given room.
    pub fn for_room(&self, room: &str) -> Vec<&ProvenanceEntry> {
        self.entries
            .iter()
            .filter(|e| e.room_id.as_deref() == Some(room))
            .collect()
    }

    /// Get entries since a timestamp.
    pub fn since(&self, ts: u64) -> Vec<&ProvenanceEntry> {
        self.entries.iter().filter(|e| e.timestamp >= ts).collect()
    }

    /// Get entries between two timestamps.
    pub fn between(&self, from: u64, to: u64) -> Vec<&ProvenanceEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= from && e.timestamp <= to)
            .collect()
    }

    /// Search entries by intent and tradeoffs text.
    pub fn search(&self, query: &str) -> Vec<&ProvenanceEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.intent.to_lowercase().contains(&q)
                    || e.tradeoffs.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Number of entries in the ledger.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Render the full ledger as markdown.
    pub fn render_full(&self) -> String {
        let mut md = String::from("# Provenance Ledger\n\n");
        for entry in &self.entries {
            md.push_str(&entry.render_markdown());
            md.push_str("\n---\n\n");
        }
        md
    }

    /// Render entries since a timestamp.
    pub fn render_since(&self, ts: u64) -> String {
        let entries = self.since(ts);
        let mut md = String::from("# Provenance Ledger (recent)\n\n");
        for entry in entries {
            md.push_str(&entry.render_markdown());
            md.push_str("\n---\n\n");
        }
        md
    }

    /// Compute statistics about the ledger.
    pub fn stats(&self) -> LedgerStats {
        let total_entries = self.entries.len();

        let mut models = std::collections::HashSet::new();
        let mut rooms = std::collections::HashSet::new();
        let mut total_cost = 0.0_f64;
        let mut tests_passed = 0usize;
        let mut total_alternatives = 0usize;

        for entry in &self.entries {
            models.insert(entry.model_used.clone());
            if let Some(room) = &entry.room_id {
                rooms.insert(room.clone());
            }
            total_cost += entry.conservation_cost;
            if entry.tests_passed {
                tests_passed += 1;
            }
            total_alternatives += entry.alternatives.len();
        }

        let tests_pass_rate = if total_entries > 0 {
            tests_passed as f64 / total_entries as f64
        } else {
            0.0
        };

        let avg_alternatives = if total_entries > 0 {
            total_alternatives as f64 / total_entries as f64
        } else {
            0.0
        };

        let mut unique_models: Vec<String> = models.into_iter().collect();
        unique_models.sort();
        let mut unique_rooms: Vec<String> = rooms.into_iter().collect();
        unique_rooms.sort();

        LedgerStats {
            total_entries,
            unique_models,
            unique_rooms,
            total_cost,
            tests_pass_rate,
            avg_alternatives,
        }
    }
}

impl Default for ProvenanceLedger {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-commit hook enforcement for provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvenanceHook {
    pub ledger: ProvenanceLedger,
    pub strict: bool,
}

impl ProvenanceHook {
    pub fn new(ledger: ProvenanceLedger) -> Self {
        Self {
            ledger,
            strict: false,
        }
    }

    /// Validate that files being committed have a provenance entry.
    pub fn validate_commit(
        &self,
        files_changed: &[&str],
    ) -> Result<Option<&ProvenanceEntry>, HookError> {
        if files_changed.is_empty() {
            return Err(HookError::NotStaged);
        }

        // Find the latest entry that mentions any of these files
        let entry = self
            .ledger
            .entries
            .iter()
            .rev()
            .find(|e| {
                files_changed
                    .iter()
                    .any(|f| e.files_changed.iter().any(|ef| ef == f))
            });

        match entry {
            Some(e) => {
                if self.strict {
                    match e.validate() {
                        Ok(()) => Ok(Some(e)),
                        Err(errs) => Err(HookError::EntryInvalid(errs)),
                    }
                } else {
                    Ok(Some(e))
                }
            }
            None => Err(HookError::NoEntryFound),
        }
    }

    /// Enforce that a provenance entry has all required fields.
    pub fn enforce(&self, entry: &ProvenanceEntry) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if entry.intent.trim().is_empty() {
            errors.push("Intent is required".to_string());
        }
        if entry.alternatives.is_empty() {
            errors.push("At least one alternative is required".to_string());
        }
        if entry.tradeoffs.trim().is_empty() {
            errors.push("Tradeoffs are required".to_string());
        }
        if entry.files_changed.is_empty() {
            errors.push("At least one file must be listed".to_string());
        }
        if !entry.alternatives.iter().any(|a| a.selected) {
            errors.push("At least one alternative must be selected".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_entry() -> ProvenanceEntry {
        let mut entry = ProvenanceEntry::new("Add authentication middleware", "gpt-4");
        entry.add_alternative("JWT tokens", "Industry standard, good tooling", true);
        entry.add_alternative("Session cookies", "Simpler but less scalable", false);
        entry.set_tradeoffs("JWT is stateless but larger payload size");
        entry.set_debt("Need to add token rotation later");
        entry.link_files(&["src/auth.rs", "src/middleware.rs"]);
        entry.set_tests_passed(true);
        entry
    }

    // --- ProvenanceEntry tests ---

    #[test]
    fn test_entry_new() {
        let entry = ProvenanceEntry::new("Do something", "gpt-4");
        assert_eq!(entry.intent, "Do something");
        assert_eq!(entry.model_used, "gpt-4");
        assert!(entry.commit_sha.is_none());
        assert!(entry.alternatives.is_empty());
        assert!(entry.files_changed.is_empty());
        assert!(!entry.tests_passed);
    }

    #[test]
    fn test_add_alternative() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.add_alternative("Option A", "Good reason", true);
        entry.add_alternative("Option B", "Another reason", false);
        assert_eq!(entry.alternatives.len(), 2);
        assert!(entry.alternatives[0].selected);
        assert!(!entry.alternatives[1].selected);
    }

    #[test]
    fn test_set_tradeoffs() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.set_tradeoffs("Speed vs memory");
        assert_eq!(entry.tradeoffs, "Speed vs memory");
    }

    #[test]
    fn test_set_debt() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.set_debt("Will need refactoring");
        assert_eq!(entry.debt, "Will need refactoring");
    }

    #[test]
    fn test_link_commit() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.link_commit("abc123");
        assert_eq!(entry.commit_sha, Some("abc123".to_string()));
    }

    #[test]
    fn test_link_room() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.link_room("room-42");
        assert_eq!(entry.room_id, Some("room-42".to_string()));
    }

    #[test]
    fn test_link_files() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.link_files(&["src/main.rs", "src/lib.rs"]);
        assert_eq!(entry.files_changed, vec!["src/main.rs", "src/lib.rs"]);
    }

    #[test]
    fn test_set_tests_passed() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.set_tests_passed(true);
        assert!(entry.tests_passed);
    }

    #[test]
    fn test_validate_valid_entry() {
        let entry = make_valid_entry();
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn test_validate_missing_intent() {
        let mut entry = ProvenanceEntry::new("", "gpt-4");
        entry.add_alternative("A", "B", true);
        entry.set_tradeoffs("something");
        let result = entry.validate();
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("Intent")));
    }

    #[test]
    fn test_validate_no_alternatives() {
        let mut entry = ProvenanceEntry::new("Do stuff", "gpt-4");
        entry.set_tradeoffs("something");
        let result = entry.validate();
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("alternative")));
    }

    #[test]
    fn test_validate_no_tradeoffs() {
        let mut entry = ProvenanceEntry::new("Do stuff", "gpt-4");
        entry.add_alternative("A", "B", true);
        let result = entry.validate();
        assert!(result.is_err());
        let errs = result.unwrap_err();
        assert!(errs.iter().any(|e| e.contains("Tradeoffs")));
    }

    #[test]
    fn test_validate_multiple_errors() {
        let entry = ProvenanceEntry::new("", "gpt-4");
        let result = entry.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().len(), 3); // intent + alternatives + tradeoffs
    }

    #[test]
    fn test_render_markdown() {
        let entry = make_valid_entry();
        let md = entry.render_markdown();
        assert!(md.contains("Provenance:"));
        assert!(md.contains("Add authentication middleware"));
        assert!(md.contains("gpt-4"));
        assert!(md.contains("JWT tokens"));
        assert!(md.contains("Session cookies"));
        assert!(md.contains("Tradeoffs"));
        assert!(md.contains("Debt"));
        assert!(md.contains("src/auth.rs"));
        assert!(md.contains("Yes"));
    }

    #[test]
    fn test_render_markdown_with_commit() {
        let mut entry = make_valid_entry();
        entry.link_commit("deadbeef");
        entry.link_room("room-1");
        let md = entry.render_markdown();
        assert!(md.contains("deadbeef"));
        assert!(md.contains("room-1"));
    }

    #[test]
    fn test_from_markdown_roundtrip() {
        let mut entry = make_valid_entry();
        entry.link_commit("abc123");
        entry.link_room("room-1");
        entry.conservation_cost = 5.5;
        let md = entry.render_markdown();
        let parsed = ProvenanceEntry::from_markdown(&md).unwrap();

        assert_eq!(parsed.intent, entry.intent);
        assert_eq!(parsed.model_used, entry.model_used);
        assert_eq!(parsed.commit_sha, entry.commit_sha);
        assert_eq!(parsed.room_id, entry.room_id);
        assert_eq!(parsed.alternatives.len(), 2);
        assert!(parsed.alternatives[0].selected);
        assert!(!parsed.alternatives[1].selected);
        assert_eq!(parsed.files_changed, entry.files_changed);
        assert_eq!(parsed.tests_passed, true);
        assert!((parsed.conservation_cost - 5.5).abs() < 0.01);
    }

    #[test]
    fn test_from_markdown_minimal() {
        let md = "# Provenance: prov-123\n\n**Intent:** Test\n\n**Model:** claude\n";
        let entry = ProvenanceEntry::from_markdown(md).unwrap();
        assert_eq!(entry.id, "prov-123");
        assert_eq!(entry.intent, "Test");
        assert_eq!(entry.model_used, "claude");
    }

    #[test]
    fn test_summary() {
        let mut entry = make_valid_entry();
        entry.link_commit("abc123");
        entry.conservation_cost = 3.5;
        let sum = entry.summary();
        assert!(sum.contains("abc123"));
        assert!(sum.contains("Add authentication middleware"));
        assert!(sum.contains("gpt-4"));
    }

    #[test]
    fn test_summary_no_commit() {
        let entry = make_valid_entry();
        let sum = entry.summary();
        assert!(sum.contains("no-commit"));
    }

    // --- Alternative tests ---

    #[test]
    fn test_alternative_new() {
        let alt = Alternative::new("JWT", "Standard approach", true);
        assert_eq!(alt.name, "JWT");
        assert_eq!(alt.reason, "Standard approach");
        assert!(alt.selected);
    }

    // --- ProvenanceLedger tests ---

    #[test]
    fn test_ledger_new() {
        let ledger = ProvenanceLedger::new();
        assert!(ledger.entries.is_empty());
        assert!(ledger.repo_path.is_none());
    }

    #[test]
    fn test_ledger_add_valid() {
        let mut ledger = ProvenanceLedger::new();
        let entry = make_valid_entry();
        assert!(ledger.add(entry).is_ok());
        assert_eq!(ledger.count(), 1);
    }

    #[test]
    fn test_ledger_add_invalid() {
        let mut ledger = ProvenanceLedger::new();
        let entry = ProvenanceEntry::new("", "gpt-4");
        assert!(ledger.add(entry).is_err());
        assert_eq!(ledger.count(), 0);
    }

    #[test]
    fn test_ledger_latest() {
        let mut ledger = ProvenanceLedger::new();
        assert!(ledger.latest().is_none());
        let e1 = make_valid_entry();
        ledger.add(e1.clone()).unwrap();
        let mut e2 = make_valid_entry();
        e2.intent = "Second entry".to_string();
        ledger.add(e2).unwrap();
        assert_eq!(ledger.latest().unwrap().intent, "Second entry");
    }

    #[test]
    fn test_ledger_for_commit() {
        let mut ledger = ProvenanceLedger::new();
        let mut e1 = make_valid_entry();
        e1.link_commit("sha1");
        let mut e2 = make_valid_entry();
        e2.link_commit("sha2");
        ledger.add(e1).unwrap();
        ledger.add(e2).unwrap();
        assert_eq!(
            ledger.for_commit("sha1").unwrap().commit_sha,
            Some("sha1".to_string())
        );
        assert!(ledger.for_commit("sha3").is_none());
    }

    #[test]
    fn test_ledger_for_room() {
        let mut ledger = ProvenanceLedger::new();
        let mut e1 = make_valid_entry();
        e1.link_room("room-a");
        let mut e2 = make_valid_entry();
        e2.link_room("room-b");
        let mut e3 = make_valid_entry();
        e3.link_room("room-a");
        ledger.add(e1).unwrap();
        ledger.add(e2).unwrap();
        ledger.add(e3).unwrap();
        assert_eq!(ledger.for_room("room-a").len(), 2);
        assert_eq!(ledger.for_room("room-b").len(), 1);
        assert!(ledger.for_room("room-c").is_empty());
    }

    #[test]
    fn test_ledger_since() {
        let mut ledger = ProvenanceLedger::new();
        let mut e1 = make_valid_entry();
        e1.timestamp = 100;
        let mut e2 = make_valid_entry();
        e2.timestamp = 200;
        let mut e3 = make_valid_entry();
        e3.timestamp = 300;
        ledger.add(e1).unwrap();
        ledger.add(e2).unwrap();
        ledger.add(e3).unwrap();
        assert_eq!(ledger.since(200).len(), 2);
        assert_eq!(ledger.since(0).len(), 3);
        assert!(ledger.since(400).is_empty());
    }

    #[test]
    fn test_ledger_between() {
        let mut ledger = ProvenanceLedger::new();
        let mut e1 = make_valid_entry();
        e1.timestamp = 100;
        let mut e2 = make_valid_entry();
        e2.timestamp = 200;
        let mut e3 = make_valid_entry();
        e3.timestamp = 300;
        ledger.add(e1).unwrap();
        ledger.add(e2).unwrap();
        ledger.add(e3).unwrap();
        assert_eq!(ledger.between(100, 200).len(), 2);
        assert_eq!(ledger.between(150, 250).len(), 1);
    }

    #[test]
    fn test_ledger_search() {
        let mut ledger = ProvenanceLedger::new();
        let mut e1 = make_valid_entry();
        e1.intent = "Add authentication".to_string();
        e1.set_tradeoffs("Security vs complexity");
        let mut e2 = make_valid_entry();
        e2.intent = "Fix logging bug".to_string();
        e2.set_tradeoffs("Performance impact");
        ledger.add(e1).unwrap();
        ledger.add(e2).unwrap();
        assert_eq!(ledger.search("authentication").len(), 1);
        assert_eq!(ledger.search("performance").len(), 1);
        assert_eq!(ledger.search("logging").len(), 1);
        assert!(ledger.search("nonexistent").is_empty());
    }

    #[test]
    fn test_ledger_search_case_insensitive() {
        let mut ledger = ProvenanceLedger::new();
        let mut e = make_valid_entry();
        e.intent = "Add Authentication".to_string();
        ledger.add(e).unwrap();
        assert_eq!(ledger.search("authentication").len(), 1);
        assert_eq!(ledger.search("AUTHENTICATION").len(), 1);
    }

    #[test]
    fn test_ledger_count() {
        let mut ledger = ProvenanceLedger::new();
        assert_eq!(ledger.count(), 0);
        ledger.add(make_valid_entry()).unwrap();
        assert_eq!(ledger.count(), 1);
        ledger.add(make_valid_entry()).unwrap();
        assert_eq!(ledger.count(), 2);
    }

    #[test]
    fn test_ledger_render_full() {
        let mut ledger = ProvenanceLedger::new();
        ledger.add(make_valid_entry()).unwrap();
        let md = ledger.render_full();
        assert!(md.contains("Provenance Ledger"));
        assert!(md.contains("---"));
    }

    #[test]
    fn test_ledger_render_since() {
        let mut ledger = ProvenanceLedger::new();
        let mut e1 = make_valid_entry();
        e1.timestamp = 100;
        let mut e2 = make_valid_entry();
        e2.timestamp = 200;
        ledger.add(e1).unwrap();
        ledger.add(e2).unwrap();
        let md = ledger.render_since(200);
        assert!(md.contains("Provenance Ledger"));
    }

    #[test]
    fn test_ledger_stats() {
        let mut ledger = ProvenanceLedger::new();
        let mut e1 = make_valid_entry();
        e1.model_used = "gpt-4".to_string();
        e1.link_room("room-a");
        e1.conservation_cost = 10.0;
        e1.set_tests_passed(true);

        let mut e2 = make_valid_entry();
        e2.model_used = "claude".to_string();
        e2.link_room("room-b");
        e2.conservation_cost = 5.0;
        e2.set_tests_passed(false);

        ledger.add(e1).unwrap();
        ledger.add(e2).unwrap();

        let stats = ledger.stats();
        assert_eq!(stats.total_entries, 2);
        assert!(stats.unique_models.contains(&"gpt-4".to_string()));
        assert!(stats.unique_models.contains(&"claude".to_string()));
        assert_eq!(stats.unique_rooms.len(), 2);
        assert!((stats.total_cost - 15.0).abs() < 0.01);
        assert!((stats.tests_pass_rate - 0.5).abs() < 0.01);
        assert!((stats.avg_alternatives - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_ledger_stats_empty() {
        let ledger = ProvenanceLedger::new();
        let stats = ledger.stats();
        assert_eq!(stats.total_entries, 0);
        assert!(stats.unique_models.is_empty());
        assert_eq!(stats.tests_pass_rate, 0.0);
        assert_eq!(stats.avg_alternatives, 0.0);
    }

    // --- ProvenanceHook tests ---

    #[test]
    fn test_hook_validate_commit_found() {
        let mut ledger = ProvenanceLedger::new();
        let mut entry = make_valid_entry();
        entry.link_files(&["src/auth.rs", "src/lib.rs"]);
        ledger.add(entry).unwrap();
        let hook = ProvenanceHook::new(ledger);
        let result = hook.validate_commit(&["src/auth.rs"]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_hook_validate_commit_not_found() {
        let mut ledger = ProvenanceLedger::new();
        let mut entry = make_valid_entry();
        entry.link_files(&["src/other.rs"]);
        ledger.add(entry).unwrap();
        let hook = ProvenanceHook::new(ledger);
        let result = hook.validate_commit(&["src/auth.rs"]);
        assert!(matches!(result, Err(HookError::NoEntryFound)));
    }

    #[test]
    fn test_hook_validate_commit_empty_files() {
        let ledger = ProvenanceLedger::new();
        let hook = ProvenanceHook::new(ledger);
        let result = hook.validate_commit(&[]);
        assert!(matches!(result, Err(HookError::NotStaged)));
    }

    #[test]
    fn test_hook_strict_mode_valid() {
        let mut ledger = ProvenanceLedger::new();
        let mut entry = make_valid_entry();
        entry.link_files(&["src/main.rs"]);
        ledger.add(entry).unwrap();
        let mut hook = ProvenanceHook::new(ledger);
        hook.strict = true;
        let result = hook.validate_commit(&["src/main.rs"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hook_strict_mode_invalid() {
        let mut ledger = ProvenanceLedger::new();
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.add_alternative("A", "B", true);
        entry.set_tradeoffs("T");
        entry.link_files(&["src/main.rs"]);
        ledger.add(entry).unwrap();
        let mut hook = ProvenanceHook::new(ledger);
        hook.strict = true;
        // The entry is valid, so this should pass
        let result = hook.validate_commit(&["src/main.rs"]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hook_enforce_valid() {
        let entry = make_valid_entry();
        let hook = ProvenanceHook::new(ProvenanceLedger::new());
        assert!(hook.enforce(&entry).is_ok());
    }

    #[test]
    fn test_hook_enforce_missing_intent() {
        let mut entry = ProvenanceEntry::new("", "gpt-4");
        entry.add_alternative("A", "B", true);
        entry.set_tradeoffs("T");
        entry.link_files(&["src/main.rs"]);
        let hook = ProvenanceHook::new(ProvenanceLedger::new());
        let result = hook.enforce(&entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_hook_enforce_no_selected_alternative() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.add_alternative("A", "B", false);
        entry.set_tradeoffs("T");
        entry.link_files(&["src/main.rs"]);
        let hook = ProvenanceHook::new(ProvenanceLedger::new());
        let result = hook.enforce(&entry);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("selected")));
    }

    #[test]
    fn test_hook_enforce_no_files() {
        let mut entry = ProvenanceEntry::new("Test", "gpt-4");
        entry.add_alternative("A", "B", true);
        entry.set_tradeoffs("T");
        let hook = ProvenanceHook::new(ProvenanceLedger::new());
        let result = hook.enforce(&entry);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("file")));
    }

    // --- Serde tests ---

    #[test]
    fn test_entry_serde_roundtrip() {
        let entry = make_valid_entry();
        let json = serde_json::to_string(&entry).unwrap();
        let de: ProvenanceEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, de);
    }

    #[test]
    fn test_ledger_serde_roundtrip() {
        let mut ledger = ProvenanceLedger::new();
        ledger.add(make_valid_entry()).unwrap();
        let json = serde_json::to_string(&ledger).unwrap();
        let de: ProvenanceLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(ledger, de);
    }

    #[test]
    fn test_stats_serde_roundtrip() {
        let mut ledger = ProvenanceLedger::new();
        ledger.add(make_valid_entry()).unwrap();
        let stats = ledger.stats();
        let json = serde_json::to_string(&stats).unwrap();
        let de: LedgerStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats, de);
    }

    #[test]
    fn test_hook_serde_roundtrip() {
        let mut ledger = ProvenanceLedger::new();
        ledger.add(make_valid_entry()).unwrap();
        let hook = ProvenanceHook::new(ledger);
        let json = serde_json::to_string(&hook).unwrap();
        let de: ProvenanceHook = serde_json::from_str(&json).unwrap();
        assert_eq!(hook, de);
    }

    #[test]
    fn test_error_serde_roundtrip() {
        let err = ProvenanceError::ValidationFailed(vec!["test error".to_string()]);
        let json = serde_json::to_string(&err).unwrap();
        let de: ProvenanceError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, de);
    }

    #[test]
    fn test_hook_error_serde_roundtrip() {
        let err = HookError::EntryInvalid(vec!["bad entry".to_string()]);
        let json = serde_json::to_string(&err).unwrap();
        let de: HookError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, de);
    }

    // --- Display tests ---

    #[test]
    fn test_provenance_error_display() {
        let err = ProvenanceError::ParseError("bad input".to_string());
        assert!(err.to_string().contains("bad input"));
    }

    #[test]
    fn test_hook_error_display() {
        let err = HookError::NoEntryFound;
        assert!(err.to_string().contains("No provenance entry"));
    }

    #[test]
    fn test_ledger_default() {
        let ledger = ProvenanceLedger::default();
        assert!(ledger.entries.is_empty());
    }
}
