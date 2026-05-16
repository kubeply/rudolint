use std::collections::{BTreeMap, BTreeSet};

use rudolint_config::Config;
use rudolint_diagnostics::{Finding, Severity};
use rudolint_dockerfile::parse_dockerfile;
use rudolint_rules::{Profile, RuleEngine, RuleInfo, RuleStatus};
use rudolint_test::read_fixture;

#[test]
fn snapshots_default_rule_findings() {
    let source = read_fixture("rules/default-basic/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default()).lint(&document);

    insta::assert_json_snapshot!(
        "default_rule_findings",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_compat_rule_findings() {
    let source = read_fixture("rules/default-basic/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Compat, Config::default()).lint(&document);

    insta::assert_json_snapshot!(
        "compat_rule_findings",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_project_native_inline_suppressions() {
    let source = read_fixture("rules/inline-suppressions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default()).lint(&document);

    insta::assert_json_snapshot!(
        "project_native_inline_suppressions",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_legacy_external_suppression_warnings() {
    let source = read_fixture("rules/legacy-suppressions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default()).lint(&document);

    insta::assert_json_snapshot!(
        "legacy_external_suppression_warnings",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl1001_legacy_suppression_fixture() {
    let source = read_fixture("rules/RDL1001.legacy-suppression/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL1001")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl1001_legacy_suppression_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3000_absolute_workdir_fixture() {
    let source = read_fixture("rules/RDL3000.absolute-workdir/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3000")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3000_absolute_workdir_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3002_final_user_not_root_fixture() {
    let fixtures = [
        "RDL3002.no-user",
        "RDL3002.switches-away-from-root",
        "RDL3002.numeric-non-root",
        "RDL3002.final-root-name",
        "RDL3002.final-root-id",
        "RDL3002.final-root-group",
    ];

    let cases = fixtures
        .into_iter()
        .map(|fixture| {
            let source = read_fixture(format!("rules/{fixture}/Dockerfile"));
            let document = parse_dockerfile(&source).expect("fixture should parse");
            let findings = RuleEngine::new(Profile::Default, Config::default())
                .lint(&document)
                .into_iter()
                .filter(|finding| finding.code == "RDL3002")
                .collect::<Vec<_>>();

            serde_json::json!({
                "fixture": fixture,
                "findings": findings,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3002_final_user_not_root_fixture",
        serde_json::to_value(&cases).expect("cases should serialize")
    );
}

#[test]
fn snapshots_rdl3006_explicit_from_tag_fixture() {
    let source = read_fixture("rules/RDL3006.explicit-from-tag/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3006")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3006_explicit_from_tag_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3007_no_latest_tag_fixture() {
    let source = read_fixture("rules/RDL3007.no-latest-tag/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3007")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3007_no_latest_tag_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3011_valid_expose_port_fixture() {
    let source = read_fixture("rules/RDL3011.expose-port-validation/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3011")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3011_valid_expose_port_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3012_healthcheck_cardinality_fixture() {
    let fixtures = [
        "RDL3012.no-healthcheck",
        "RDL3012.single-healthcheck-cmd",
        "RDL3012.single-healthcheck-none",
        "RDL3012.duplicate-healthcheck",
    ];

    let cases = fixtures
        .into_iter()
        .map(|fixture| {
            let source = read_fixture(format!("rules/{fixture}/Dockerfile"));
            let document = parse_dockerfile(&source).expect("fixture should parse");
            let findings = RuleEngine::new(Profile::Default, Config::default())
                .lint(&document)
                .into_iter()
                .filter(|finding| finding.code == "RDL3012")
                .collect::<Vec<_>>();

            serde_json::json!({
                "fixture": fixture,
                "findings": findings,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3012_healthcheck_cardinality_fixture",
        serde_json::to_value(&cases).expect("cases should serialize")
    );
}

#[test]
fn snapshots_rdl3020_prefer_copy_fixture() {
    let source = read_fixture("rules/RDL3020.prefer-copy/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3020")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3020_prefer_copy_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3024_unique_stage_names_fixture() {
    let source = read_fixture("rules/RDL3024.unique-stage-names/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3024")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3024_unique_stage_names_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3025_json_entrypoints_fixture() {
    let source = read_fixture("rules/RDL3025.json-entrypoints/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let engine = RuleEngine::new(Profile::Default, Config::default());
    let findings = engine
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3025")
        .collect::<Vec<_>>();
    let fixes = engine
        .fixes(&document)
        .into_iter()
        .filter(|fix| fix.title.contains("exec/JSON form"))
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3025_json_entrypoints_fixture",
        serde_json::json!({
            "findings": findings,
            "fixes": fixes,
        })
    );
}

#[test]
fn snapshots_rule_selection_matrix() {
    let source = read_fixture("rules/default-basic/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");

    let default_engine = RuleEngine::new(Profile::Default, Config::default());
    let compat_engine = RuleEngine::new(Profile::Compat, Config::default());

    let ignore_config = Config {
        ignore: BTreeSet::from(["RDL3000".to_string()]),
        ..Config::default()
    };

    let severity_config = Config {
        severity: BTreeMap::from([("RDL3007".to_string(), Severity::Error)]),
        ..Config::default()
    };

    let snapshot = serde_json::json!({
        "profile_findings": {
            "default": finding_codes(&default_engine.lint(&document)),
            "compat": finding_codes(&compat_engine.lint(&document)),
        },
        "implemented_catalog": {
            "default": implemented_codes(&default_engine.catalog()),
            "compat": implemented_codes(&compat_engine.catalog()),
        },
        "config_filters": {
            "ignore_rdl3000": finding_codes(
                &RuleEngine::new(Profile::Default, ignore_config).lint(&document)
            ),
            "severity_override_rdl3007": finding_codes(
                &RuleEngine::new(Profile::Compat, severity_config).lint(&document)
            ),
        }
    });

    insta::assert_json_snapshot!("rule_selection_matrix", snapshot);
}

#[test]
fn snapshots_rule_metadata_contract() {
    let engine = RuleEngine::new(Profile::Default, Config::default());
    let metadata = engine
        .catalog()
        .into_iter()
        .filter(|rule| rule.status == RuleStatus::Implemented)
        .map(|rule| {
            let metadata = rule.metadata;
            serde_json::json!({
                "code": metadata.code,
                "name": metadata.name,
                "summary": metadata.summary,
                "default_severity": metadata.default_severity,
                "profile": metadata.profile.as_str(),
                "category": metadata.category.as_str(),
                "status": metadata.status.to_string(),
                "docs_url": metadata.docs_url,
                "fix": metadata.fix.as_str(),
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!("rule_metadata_contract", metadata);
}

fn finding_codes(findings: &[Finding]) -> Vec<serde_json::Value> {
    findings
        .iter()
        .map(|finding| {
            serde_json::json!({
                "code": finding.code,
                "severity": finding.severity,
            })
        })
        .collect()
}

fn implemented_codes(catalog: &[RuleInfo]) -> Vec<&'static str> {
    catalog
        .iter()
        .filter(|rule| rule.status == RuleStatus::Implemented)
        .map(|rule| rule.code)
        .collect()
}
