use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use rudolint_config::Config;
use rudolint_rules::{FixAvailability, Profile, RuleEngine, RuleStatus};

#[test]
fn catalog_codes_are_unique_and_have_metadata() {
    let catalog = RuleEngine::new(Profile::Default, Config::default()).catalog();
    let mut seen = BTreeSet::new();

    for rule in catalog {
        assert!(seen.insert(rule.code), "duplicate rule code {}", rule.code);
        assert_eq!(rule.code, rule.metadata.code);
        assert!(!rule.metadata.name.is_empty(), "{} missing name", rule.code);
        assert!(
            !rule.metadata.summary.is_empty(),
            "{} missing summary",
            rule.code
        );
        assert!(
            !rule.metadata.docs_url.is_empty(),
            "{} missing docs URL",
            rule.code
        );
    }
}

#[test]
fn implemented_rules_have_docs_pages() {
    let catalog = RuleEngine::new(Profile::Default, Config::default()).catalog();

    for rule in catalog
        .iter()
        .filter(|rule| rule.status == RuleStatus::Implemented)
    {
        let path = rule_docs_path(rule.code);
        let docs = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} is missing docs at {path:?}: {error}", rule.code));

        assert!(
            !docs.trim().is_empty(),
            "{} docs at {path:?} must not be empty",
            rule.code
        );
    }
}

#[test]
fn implemented_rules_and_docs_are_synchronized() {
    let catalog = RuleEngine::new(Profile::Default, Config::default()).catalog();
    let implemented_codes = catalog
        .iter()
        .filter(|rule| rule.status == RuleStatus::Implemented)
        .map(|rule| rule.code.to_string())
        .collect::<BTreeSet<_>>();
    let documented_codes = documented_rule_codes();

    for code in &implemented_codes {
        assert!(documented_codes.contains(code), "{code} is missing docs");
    }

    for code in &documented_codes {
        assert!(
            catalog.iter().any(|rule| rule.code == code),
            "{code} docs do not match a catalog rule"
        );
    }
}

#[test]
fn implemented_rules_appear_in_rule_matrix() {
    let catalog = RuleEngine::new(Profile::Default, Config::default()).catalog();
    let matrix_codes = rule_matrix_implemented_codes();

    for rule in catalog
        .iter()
        .filter(|rule| rule.status == RuleStatus::Implemented)
    {
        assert!(
            matrix_codes.contains(rule.code),
            "{} is missing from docs/rule-roadmap.md implemented matrix",
            rule.code
        );
    }
}

#[test]
fn implemented_rule_docs_declare_fix_availability() {
    let catalog = RuleEngine::new(Profile::Default, Config::default()).catalog();
    let docs_dir = workspace_root().join("docs/rules");

    for rule in catalog
        .iter()
        .filter(|rule| rule.status == RuleStatus::Implemented)
    {
        let docs = fs::read_to_string(docs_dir.join(format!("{}.md", rule.code)))
            .unwrap_or_else(|error| panic!("failed to read docs for {}: {error}", rule.code));
        let autofix_line = docs
            .lines()
            .find(|line| line.trim_start().starts_with("- Autofix:"))
            .unwrap_or_else(|| panic!("{} docs must declare autofix behavior", rule.code));
        assert!(
            !autofix_line.contains("Autofix: none yet"),
            "{} docs must include a no-fix rationale instead of a placeholder",
            rule.code
        );

        match rule.metadata.fix {
            FixAvailability::Safe => assert!(
                autofix_line.contains("safe automatic"),
                "{} safe fix docs must say safe automatic",
                rule.code
            ),
            FixAvailability::Manual => assert!(
                autofix_line.contains("manual"),
                "{} manual fix docs must say manual",
                rule.code
            ),
            FixAvailability::None => assert!(
                autofix_line.contains("no safe automatic fix"),
                "{} no-fix docs must include a rationale",
                rule.code
            ),
        }
    }
}

fn documented_rule_codes() -> BTreeSet<String> {
    let docs_dir = workspace_root().join("docs/rules");
    fs::read_dir(docs_dir)
        .expect("rules docs directory should be readable")
        .filter_map(|entry| {
            let path = entry.expect("rules docs entry should be readable").path();
            let stem = path.file_stem()?.to_str()?;
            (stem.starts_with("RD") || stem.starts_with("RS")).then(|| stem.to_string())
        })
        .collect()
}

fn rule_docs_path(code: &str) -> PathBuf {
    workspace_root()
        .join("docs/rules")
        .join(format!("{code}.md"))
}

fn rule_matrix_implemented_codes() -> BTreeSet<String> {
    let roadmap = fs::read_to_string(workspace_root().join("docs/rule-roadmap.md"))
        .expect("rule roadmap should be readable");
    let mut in_implemented_section = false;
    let mut codes = BTreeSet::new();

    for line in roadmap.lines() {
        if line == "## Implemented V1 Surface" {
            in_implemented_section = true;
            continue;
        }

        if in_implemented_section && line.starts_with("## ") {
            break;
        }

        if !in_implemented_section || !line.starts_with("| `") {
            continue;
        }

        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some((code, _)) = rest.split_once('`') else {
            continue;
        };

        codes.insert(code.to_string());
    }

    codes
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("rudolint-rules should live under crates/rudolint-rules")
        .to_path_buf()
}
