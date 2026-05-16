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
fn snapshots_rdl3001_disallowed_container_commands_fixture() {
    let source = read_fixture("rules/RDL3001.disallowed-container-commands/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3001")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3001_disallowed_container_commands_fixture",
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
fn snapshots_rdl3003_use_workdir_for_cd_fixture() {
    let source = read_fixture("rules/RDL3003.use-workdir-for-cd/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3003")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3003_use_workdir_for_cd_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3004_no_sudo_fixture() {
    let source = read_fixture("rules/RDL3004.no-sudo/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3004")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3004_no_sudo_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
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
fn snapshots_rdl3008_pin_apt_get_install_versions_fixture() {
    let source = read_fixture("rules/RDL3008.pin-apt-get-install-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3008")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3008_pin_apt_get_install_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3009_clean_apt_lists_fixture() {
    let source = read_fixture("rules/RDL3009.clean-apt-lists/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3009")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3009_clean_apt_lists_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3010_use_add_for_archives_fixture() {
    let source = read_fixture("rules/RDL3010.use-add-for-archives/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3010")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3010_use_add_for_archives_fixture",
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
fn snapshots_rdl3013_pin_pip_versions_fixture() {
    let source = read_fixture("rules/RDL3013.pin-pip-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3013")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3013_pin_pip_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3014_apt_get_install_assume_yes_fixture() {
    let source = read_fixture("rules/RDL3014.apt-get-install-assume-yes/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3014")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3014_apt_get_install_assume_yes_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3015_apt_get_no_install_recommends_fixture() {
    let source = read_fixture("rules/RDL3015.apt-get-no-install-recommends/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3015")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3015_apt_get_no_install_recommends_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3016_pin_npm_versions_fixture() {
    let source = read_fixture("rules/RDL3016.pin-npm-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3016")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3016_pin_npm_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3018_pin_apk_versions_fixture() {
    let source = read_fixture("rules/RDL3018.pin-apk-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3018")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3018_pin_apk_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3019_apk_add_no_cache_fixture() {
    let source = read_fixture("rules/RDL3019.apk-add-no-cache/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3019")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3019_apk_add_no_cache_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
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
fn snapshots_rdl3021_copy_multiple_destination_slash_fixture() {
    let source = read_fixture("rules/RDL3021.copy-multiple-destination-slash/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3021")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3021_copy_multiple_destination_slash_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3022_copy_from_previous_stage_fixture() {
    let source = read_fixture("rules/RDL3022.copy-from-previous-stage/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3022")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3022_copy_from_previous_stage_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3023_copy_from_own_stage_fixture() {
    let source = read_fixture("rules/RDL3023.copy-from-own-stage/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3023")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3023_copy_from_own_stage_fixture",
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
fn snapshots_rdl3026_trusted_registries_fixture() {
    let source = read_fixture("rules/RDL3026.trusted-registries/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        trusted_registries: vec!["ghcr.io".to_string(), "localhost:5000".to_string()],
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3026")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3026_trusted_registries_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3027_use_apt_get_fixture() {
    let source = read_fixture("rules/RDL3027.use-apt-get/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3027")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3027_use_apt_get_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3028_pin_gem_versions_fixture() {
    let source = read_fixture("rules/RDL3028.pin-gem-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3028")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3028_pin_gem_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3029_no_from_platform_flag_fixture() {
    let source = read_fixture("rules/RDL3029.no-from-platform-flag/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3029")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3029_no_from_platform_flag_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3030_yum_install_assume_yes_fixture() {
    let source = read_fixture("rules/RDL3030.yum-install-assume-yes/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3030")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3030_yum_install_assume_yes_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3032_yum_clean_all_fixture() {
    let source = read_fixture("rules/RDL3032.yum-clean-all/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3032")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3032_yum_clean_all_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3033_pin_yum_versions_fixture() {
    let source = read_fixture("rules/RDL3033.pin-yum-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3033")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3033_pin_yum_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl3034_zypper_install_assume_yes_fixture() {
    let source = read_fixture("rules/RDL3034.zypper-install-assume-yes/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL3034")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl3034_zypper_install_assume_yes_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdl4000_deprecated_maintainer_fixture() {
    let source = read_fixture("rules/RDL4000.deprecated-maintainer/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let engine = RuleEngine::new(Profile::Default, Config::default());
    let findings = engine
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDL4000")
        .collect::<Vec<_>>();
    let fixes = engine
        .fixes(&document)
        .into_iter()
        .filter(|fix| fix.title == "replace MAINTAINER with OCI authors label")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl4000_deprecated_maintainer_fixture",
        serde_json::json!({
            "findings": findings,
            "fixes": fixes,
        })
    );
}

#[test]
fn snapshots_rdl4003_cmd_cardinality_fixture() {
    let fixtures = [
        "RDL4003.no-cmd",
        "RDL4003.single-cmd",
        "RDL4003.duplicate-cmd",
    ];

    let cases = fixtures
        .into_iter()
        .map(|fixture| {
            let source = read_fixture(format!("rules/{fixture}/Dockerfile"));
            let document = parse_dockerfile(&source).expect("fixture should parse");
            let findings = RuleEngine::new(Profile::Default, Config::default())
                .lint(&document)
                .into_iter()
                .filter(|finding| finding.code == "RDL4003")
                .collect::<Vec<_>>();

            serde_json::json!({
                "fixture": fixture,
                "findings": findings,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl4003_cmd_cardinality_fixture",
        serde_json::to_value(&cases).expect("cases should serialize")
    );
}

#[test]
fn snapshots_rdl4004_entrypoint_cardinality_fixture() {
    let fixtures = [
        "RDL4004.no-entrypoint",
        "RDL4004.single-entrypoint",
        "RDL4004.duplicate-entrypoint",
    ];

    let cases = fixtures
        .into_iter()
        .map(|fixture| {
            let source = read_fixture(format!("rules/{fixture}/Dockerfile"));
            let document = parse_dockerfile(&source).expect("fixture should parse");
            let findings = RuleEngine::new(Profile::Default, Config::default())
                .lint(&document)
                .into_iter()
                .filter(|finding| finding.code == "RDL4004")
                .collect::<Vec<_>>();

            serde_json::json!({
                "fixture": fixture,
                "findings": findings,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdl4004_entrypoint_cardinality_fixture",
        serde_json::to_value(&cases).expect("cases should serialize")
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
