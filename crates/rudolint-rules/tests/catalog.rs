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
fn rule_matrix_entries_are_known_catalog_rules() {
    let catalog = RuleEngine::new(Profile::Default, Config::default()).catalog();
    let catalog_codes = catalog
        .iter()
        .map(|rule| rule.code.to_string())
        .collect::<BTreeSet<_>>();

    for code in rule_matrix_codes() {
        assert!(
            catalog_codes.contains(&code),
            "{code} appears in docs/rule-roadmap.md but is not in the catalog"
        );
    }
}

#[test]
fn rule_matrix_profile_coverage_matches_catalog() {
    let default_codes = implemented_catalog_codes(Profile::Default);
    let compat_codes = implemented_catalog_codes(Profile::HadolintCompat);

    for row in rule_matrix_section_rows("## Implemented V1 Surface") {
        let expected_profiles = match (
            default_codes.contains(row.code.as_str()),
            compat_codes.contains(row.code.as_str()),
        ) {
            (true, true) => "`default`, `hadolint-compat`",
            (true, false) => "`default`",
            (false, true) => "`hadolint-compat`",
            (false, false) => {
                panic!(
                    "{} appears in the implemented matrix but no profile enables it",
                    row.code
                )
            }
        };

        assert_eq!(
            row.enabled_profiles, expected_profiles,
            "{} has stale profile coverage in docs/rule-roadmap.md",
            row.code
        );
    }
}

#[test]
fn implemented_rule_matrix_entries_have_audited_negative_coverage() {
    for row in rule_matrix_section_rows("## Implemented V1 Surface") {
        assert!(
            matches!(row.negative_fixture.as_str(), "yes" | "shared"),
            "{} must declare focused or shared negative/noise coverage",
            row.code
        );
    }
}

#[test]
fn planned_shell_rules_stay_out_of_implemented_matrix() {
    let catalog = RuleEngine::new(Profile::Default, Config::default()).catalog();
    let implemented_matrix_codes = rule_matrix_implemented_codes();
    let planned_shell_matrix_codes = rule_matrix_section_codes("## Planned Future Shell Rules");

    for rule in catalog
        .iter()
        .filter(|rule| rule.status == RuleStatus::Planned && rule.code.starts_with("RSC"))
    {
        assert!(
            !implemented_matrix_codes.contains(rule.code),
            "{} is planned shell coverage and must not appear in the implemented v1 matrix",
            rule.code
        );
        assert!(
            planned_shell_matrix_codes.contains(rule.code),
            "{} is planned shell coverage and must stay in the planned shell matrix",
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
    rule_matrix_section_codes("## Implemented V1 Surface")
}

fn implemented_catalog_codes(profile: Profile) -> BTreeSet<String> {
    RuleEngine::new(profile, Config::default())
        .catalog()
        .into_iter()
        .filter(|rule| rule.status == RuleStatus::Implemented)
        .map(|rule| rule.code.to_string())
        .collect()
}

fn rule_matrix_section_codes(heading: &str) -> BTreeSet<String> {
    rule_matrix_section_rows(heading)
        .into_iter()
        .map(|row| row.code)
        .collect()
}

#[derive(Debug)]
struct RuleMatrixRow {
    code: String,
    enabled_profiles: String,
    negative_fixture: String,
}

fn rule_matrix_section_rows(heading: &str) -> Vec<RuleMatrixRow> {
    let roadmap = fs::read_to_string(workspace_root().join("docs/rule-roadmap.md"))
        .expect("rule roadmap should be readable");
    let mut in_section = false;
    let mut rows = Vec::new();

    for line in roadmap.lines() {
        if line == heading {
            in_section = true;
            continue;
        }

        if in_section && line.starts_with("## ") {
            break;
        }

        if !in_section {
            continue;
        }

        if let Some(row) = rule_matrix_row(line) {
            rows.push(row);
        }
    }

    rows
}

fn rule_matrix_codes() -> BTreeSet<String> {
    let roadmap = fs::read_to_string(workspace_root().join("docs/rule-roadmap.md"))
        .expect("rule roadmap should be readable");

    roadmap
        .lines()
        .filter_map(rule_matrix_row)
        .map(|row| row.code)
        .collect()
}

fn rule_matrix_row(line: &str) -> Option<RuleMatrixRow> {
    let cells = line
        .trim()
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>();

    let code_cell = cells.first()?;
    let code = code_cell.strip_prefix('`')?.strip_suffix('`')?;
    let enabled_profiles = cells.get(2)?;
    let negative_fixture = cells.get(6)?;

    Some(RuleMatrixRow {
        code: code.to_string(),
        enabled_profiles: (*enabled_profiles).to_string(),
        negative_fixture: (*negative_fixture).to_string(),
    })
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("rudolint-rules should live under crates/rudolint-rules")
        .to_path_buf()
}
