use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use rudolint_config::Config;
use rudolint_rules::{Profile, RuleEngine, RuleStatus};

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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("rudolint-rules should live under crates/rudolint-rules")
        .to_path_buf()
}
