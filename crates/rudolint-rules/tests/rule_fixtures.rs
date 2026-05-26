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
    let findings = RuleEngine::new(Profile::HadolintCompat, Config::default()).lint(&document);

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
fn hadolint_compat_accepts_hadolint_suppression_comments() {
    let source = read_fixture("rules/legacy-suppressions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::HadolintCompat, Config::default()).lint(&document);

    assert!(
        findings.iter().all(|finding| finding.code != "RUD1001"),
        "hadolint-compat should not warn on hadolint ignore comments"
    );
    assert!(
        findings.iter().all(|finding| finding.code != "DL3007"),
        "hadolint-compat should honor hadolint ignore comments"
    );
}

#[test]
fn hadolint_compat_suppresses_source_located_multiline_shell_findings() {
    let source = r#"# hadolint ignore=SC2086, SC2010, DL3042
RUN --mount=type=cache,target=/tmp/cache \
    if [[ ${INSTALL_DISTRIBUTIONS_FROM_CONTEXT} == "true" ]]; then \
        echo ok; \
    fi
"#;
    let document = parse_dockerfile(source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::HadolintCompat, Config::default()).lint(&document);

    assert!(
        findings.iter().all(|finding| finding.code != "SC2086"),
        "hadolint-compat should suppress shell findings located after the RUN start line"
    );
}

#[test]
fn shell_quote_checks_skip_dockerfile_declared_variables() {
    let source = r#"FROM alpine:3.20
ARG AIRFLOW_HOME
ARG PGBOUNCER_VERSION
RUN mkdir -p ${AIRFLOW_HOME}
RUN tar -xzvf pgbouncer-$PGBOUNCER_VERSION.tar.gz
RUN echo $UNKNOWN_VALUE
"#;
    let document = parse_dockerfile(source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::HadolintCompat, Config::default()).lint(&document);
    let sc2086_lines = findings
        .iter()
        .filter(|finding| finding.code == "SC2086")
        .map(Finding::line)
        .collect::<Vec<_>>();

    assert_eq!(
        sc2086_lines,
        vec![6],
        "SC2086 should match Hadolint by ignoring declared Dockerfile variables"
    );
}

#[test]
fn snapshots_real_world_corpus_findings() {
    let cases = [
        "alpine-packages",
        "buildkit-cache-mount",
        "buildkit-secret-mount",
        "buildkit-ssh-mount",
        "debian-packages",
        "generated-labels",
        "heredoc",
        "multi-platform-build",
        "multi-stage-app",
    ]
    .into_iter()
    .map(|fixture| {
        let path = format!("corpus/real-world/{fixture}/Dockerfile");
        let source = read_fixture(&path);
        let document = parse_dockerfile(&source).expect("fixture should parse");
        let findings = RuleEngine::new(Profile::Default, Config::default())
            .lint_path(std::path::Path::new(&path), &document);

        serde_json::json!({
            "fixture": fixture,
            "findings": findings,
        })
    })
    .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "real_world_corpus_findings",
        serde_json::to_value(&cases).expect("cases should serialize")
    );
}

#[test]
fn real_world_noise_fixtures_do_not_trigger_findings() {
    let profiles = [Profile::Default, Profile::HadolintCompat];

    for fixture in [
        "noise-clean-runtime",
        "noise-directives-and-comments",
        "noise-metadata-runtime",
    ] {
        let path = format!("corpus/real-world/{fixture}/Dockerfile");
        let source = read_fixture(&path);
        let document = parse_dockerfile(&source).expect("fixture should parse");

        for profile in profiles {
            let findings = RuleEngine::new(profile, Config::default())
                .lint_path(std::path::Path::new(&path), &document);

            assert!(
                findings.is_empty(),
                "{fixture} should not trigger findings in {} profile: {findings:#?}",
                profile.policy().as_str()
            );
        }
    }
}

#[test]
fn snapshots_initial_shell_rule_findings() {
    let source = read_fixture("rules/SC.initial-shell-rules/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let shell_codes = BTreeSet::from([
        "SC2002", "SC2015", "SC2046", "SC2086", "SC2155", "SC2164", "SC2181",
    ]);
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| shell_codes.contains(finding.code.as_str()))
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "initial_shell_rule_findings",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rud1001_legacy_suppression_fixture() {
    let source = read_fixture("rules/RUD1001.legacy-suppression/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RUD1001")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rud1001_legacy_suppression_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3001_disallowed_container_commands_fixture() {
    let source = read_fixture("rules/DL3001.disallowed-container-commands/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3001")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3001_disallowed_container_commands_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3000_absolute_workdir_fixture() {
    let source = read_fixture("rules/DL3000.absolute-workdir/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3000")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3000_absolute_workdir_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3002_final_user_not_root_fixture() {
    let fixtures = [
        "DL3002.no-user",
        "DL3002.switches-away-from-root",
        "DL3002.numeric-non-root",
        "DL3002.final-root-name",
        "DL3002.final-root-id",
        "DL3002.final-root-group",
    ];

    let cases = fixtures
        .into_iter()
        .map(|fixture| {
            let source = read_fixture(format!("rules/{fixture}/Dockerfile"));
            let document = parse_dockerfile(&source).expect("fixture should parse");
            let findings = RuleEngine::new(Profile::Default, Config::default())
                .lint(&document)
                .into_iter()
                .filter(|finding| finding.code == "DL3002")
                .collect::<Vec<_>>();

            serde_json::json!({
                "fixture": fixture,
                "findings": findings,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3002_final_user_not_root_fixture",
        serde_json::to_value(&cases).expect("cases should serialize")
    );
}

#[test]
fn snapshots_dl3003_use_workdir_for_cd_fixture() {
    let source = read_fixture("rules/DL3003.use-workdir-for-cd/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3003")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3003_use_workdir_for_cd_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3004_no_sudo_fixture() {
    let source = read_fixture("rules/DL3004.no-sudo/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3004")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3004_no_sudo_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3006_explicit_from_tag_fixture() {
    let source = read_fixture("rules/DL3006.explicit-from-tag/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3006")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3006_explicit_from_tag_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3007_no_latest_tag_fixture() {
    let source = read_fixture("rules/DL3007.no-latest-tag/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3007")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3007_no_latest_tag_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3008_pin_apt_get_install_versions_fixture() {
    let source = read_fixture("rules/DL3008.pin-apt-get-install-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3008")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3008_pin_apt_get_install_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3009_clean_apt_lists_fixture() {
    let source = read_fixture("rules/DL3009.clean-apt-lists/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3009")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3009_clean_apt_lists_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3010_use_add_for_archives_fixture() {
    let source = read_fixture("rules/DL3010.use-add-for-archives/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3010")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3010_use_add_for_archives_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3011_valid_expose_port_fixture() {
    let source = read_fixture("rules/DL3011.expose-port-validation/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3011")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3011_valid_expose_port_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3012_healthcheck_cardinality_fixture() {
    let fixtures = [
        "DL3012.no-healthcheck",
        "DL3012.single-healthcheck-cmd",
        "DL3012.single-healthcheck-none",
        "DL3012.duplicate-healthcheck",
    ];

    let cases = fixtures
        .into_iter()
        .map(|fixture| {
            let source = read_fixture(format!("rules/{fixture}/Dockerfile"));
            let document = parse_dockerfile(&source).expect("fixture should parse");
            let findings = RuleEngine::new(Profile::Default, Config::default())
                .lint(&document)
                .into_iter()
                .filter(|finding| finding.code == "DL3012")
                .collect::<Vec<_>>();

            serde_json::json!({
                "fixture": fixture,
                "findings": findings,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3012_healthcheck_cardinality_fixture",
        serde_json::to_value(&cases).expect("cases should serialize")
    );
}

#[test]
fn snapshots_dl3013_pin_pip_versions_fixture() {
    let source = read_fixture("rules/DL3013.pin-pip-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3013")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3013_pin_pip_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3014_apt_get_install_assume_yes_fixture() {
    let source = read_fixture("rules/DL3014.apt-get-install-assume-yes/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3014")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3014_apt_get_install_assume_yes_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3015_apt_get_no_install_recommends_fixture() {
    let source = read_fixture("rules/DL3015.apt-get-no-install-recommends/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3015")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3015_apt_get_no_install_recommends_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3016_pin_npm_versions_fixture() {
    let source = read_fixture("rules/DL3016.pin-npm-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3016")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3016_pin_npm_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3018_pin_apk_versions_fixture() {
    let source = read_fixture("rules/DL3018.pin-apk-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3018")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3018_pin_apk_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3019_apk_add_no_cache_fixture() {
    let source = read_fixture("rules/DL3019.apk-add-no-cache/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3019")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3019_apk_add_no_cache_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3020_prefer_copy_fixture() {
    let source = read_fixture("rules/DL3020.prefer-copy/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3020")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3020_prefer_copy_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3021_copy_multiple_destination_slash_fixture() {
    let source = read_fixture("rules/DL3021.copy-multiple-destination-slash/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3021")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3021_copy_multiple_destination_slash_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn copy_heredocs_do_not_treat_bodies_as_copy_operands() {
    let source = r#"FROM alpine:3.20
COPY <<"SCRIPT" /usr/local/bin/generated
#!/usr/bin/env sh
tar -xf app.tar.gz
echo "$UNQUOTED"
SCRIPT
COPY <<FIRST <<SECOND /etc/generated/
first body
FIRST
second body
SECOND
COPY --chmod=755 \
  <<'SCRIPT' /usr/local/bin/continued
#!/usr/bin/env sh
echo continued
SCRIPT
COPY <<FIRST \
  <<SECOND /etc/continued-generated/
first continued body
FIRST
second continued body
SECOND
"#;
    let document = parse_dockerfile(source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default()).lint(&document);
    let blocked_codes = findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.code.as_str(),
                "DL3010" | "DL3020" | "DL3021" | "DL3045"
            )
        })
        .map(|finding| finding.code.as_str())
        .collect::<Vec<_>>();

    assert!(
        blocked_codes.is_empty(),
        "COPY heredoc bodies should not be parsed as operands: {blocked_codes:?}"
    );
}

#[test]
fn snapshots_dl3022_copy_from_previous_stage_fixture() {
    let source = read_fixture("rules/DL3022.copy-from-previous-stage/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3022")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3022_copy_from_previous_stage_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3023_copy_from_own_stage_fixture() {
    let source = read_fixture("rules/DL3023.copy-from-own-stage/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3023")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3023_copy_from_own_stage_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3024_unique_stage_names_fixture() {
    let source = read_fixture("rules/DL3024.unique-stage-names/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3024")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3024_unique_stage_names_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3025_json_entrypoints_fixture() {
    let source = read_fixture("rules/DL3025.json-entrypoints/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let engine = RuleEngine::new(Profile::Default, Config::default());
    let findings = engine
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3025")
        .collect::<Vec<_>>();
    let fixes = engine
        .fixes(&document)
        .into_iter()
        .filter(|fix| fix.title.contains("exec/JSON form"))
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3025_json_entrypoints_fixture",
        serde_json::json!({
            "findings": findings,
            "fixes": fixes,
        })
    );
}

#[test]
fn snapshots_dl3026_trusted_registries_fixture() {
    let source = read_fixture("rules/DL3026.trusted-registries/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        trusted_registries: vec!["ghcr.io".to_string(), "localhost:5000".to_string()],
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3026")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3026_trusted_registries_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3027_use_apt_get_fixture() {
    let source = read_fixture("rules/DL3027.use-apt-get/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3027")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3027_use_apt_get_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3028_pin_gem_versions_fixture() {
    let source = read_fixture("rules/DL3028.pin-gem-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3028")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3028_pin_gem_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3029_no_from_platform_flag_fixture() {
    let source = read_fixture("rules/DL3029.no-from-platform-flag/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3029")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3029_no_from_platform_flag_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3030_yum_install_assume_yes_fixture() {
    let source = read_fixture("rules/DL3030.yum-install-assume-yes/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3030")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3030_yum_install_assume_yes_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3032_yum_clean_all_fixture() {
    let source = read_fixture("rules/DL3032.yum-clean-all/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3032")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3032_yum_clean_all_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3033_pin_yum_versions_fixture() {
    let source = read_fixture("rules/DL3033.pin-yum-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3033")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3033_pin_yum_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3034_zypper_install_assume_yes_fixture() {
    let source = read_fixture("rules/DL3034.zypper-install-assume-yes/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3034")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3034_zypper_install_assume_yes_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3035_no_zypper_dist_upgrade_fixture() {
    let source = read_fixture("rules/DL3035.no-zypper-dist-upgrade/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3035")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3035_no_zypper_dist_upgrade_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3036_zypper_clean_fixture() {
    let source = read_fixture("rules/DL3036.zypper-clean/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3036")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3036_zypper_clean_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3037_pin_zypper_versions_fixture() {
    let source = read_fixture("rules/DL3037.pin-zypper-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3037")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3037_pin_zypper_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3038_dnf_install_assume_yes_fixture() {
    let source = read_fixture("rules/DL3038.dnf-install-assume-yes/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3038")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3038_dnf_install_assume_yes_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3040_dnf_clean_all_fixture() {
    let source = read_fixture("rules/DL3040.dnf-clean-all/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3040")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3040_dnf_clean_all_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3041_pin_dnf_versions_fixture() {
    let source = read_fixture("rules/DL3041.pin-dnf-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3041")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3041_pin_dnf_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3042_pip_no_cache_dir_fixture() {
    let source = read_fixture("rules/DL3042.pip-no-cache-dir/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3042")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3042_pip_no_cache_dir_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3043_no_onbuild_trigger_fixture() {
    let source = read_fixture("rules/DL3043.no-onbuild-trigger/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3043")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3043_no_onbuild_trigger_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3044_no_env_self_reference_fixture() {
    let source = read_fixture("rules/DL3044.no-env-self-reference/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3044")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3044_no_env_self_reference_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn dl3044_allows_env_self_promotion_and_arg_promotion() {
    let source = r#"FROM alpine:3.20
ARG PROMOTED
ENV SELF=${SELF}
ENV PROMOTED=${PROMOTED}
ENV NEW_VALUE=value PATH=$NEW_VALUE/bin:$PATH
"#;
    let document = parse_dockerfile(source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default()).lint(&document);
    let dl3044_lines = findings
        .iter()
        .filter(|finding| finding.code == "DL3044")
        .map(Finding::line)
        .collect::<Vec<_>>();

    assert_eq!(
        dl3044_lines,
        vec![5],
        "DL3044 should only flag references to other variables defined in the same ENV"
    );
}

#[test]
fn snapshots_dl3045_copy_relative_without_workdir_fixture() {
    let source = read_fixture("rules/DL3045.copy-relative-without-workdir/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3045")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3045_copy_relative_without_workdir_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3046_useradd_no_log_init_fixture() {
    let source = read_fixture("rules/DL3046.useradd-no-log-init/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3046")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3046_useradd_no_log_init_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3047_wget_progress_fixture() {
    let source = read_fixture("rules/DL3047.wget-progress/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3047")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3047_wget_progress_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3048_valid_label_key_fixture() {
    let source = read_fixture("rules/DL3048.valid-label-key/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3048")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3048_valid_label_key_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3049_missing_required_labels_fixture() {
    let source = read_fixture("rules/DL3049.missing-required-labels/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
            (
                "org.opencontainers.image.source".to_string(),
                "url".to_string(),
            ),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3049")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3049_missing_required_labels_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3050_no_superfluous_labels_fixture() {
    let source = read_fixture("rules/DL3050.no-superfluous-labels/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
            (
                "org.opencontainers.image.source".to_string(),
                "url".to_string(),
            ),
        ]),
        strict_labels: true,
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3050")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3050_no_superfluous_labels_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3051_no_empty_labels_fixture() {
    let source = read_fixture("rules/DL3051.no-empty-labels/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
            (
                "org.opencontainers.image.source".to_string(),
                "url".to_string(),
            ),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3051")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3051_no_empty_labels_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3052_valid_url_labels_fixture() {
    let source = read_fixture("rules/DL3052.valid-url-labels/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.source".to_string(),
                "url".to_string(),
            ),
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3052")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3052_valid_url_labels_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3053_valid_rfc3339_labels_fixture() {
    let source = read_fixture("rules/DL3053.valid-rfc3339-labels/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.created".to_string(),
                "rfc3339".to_string(),
            ),
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3053")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3053_valid_rfc3339_labels_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3054_spdx_labels_validation_fixture() {
    let source = read_fixture("rules/DL3054.spdx-labels-validation/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.licenses".to_string(),
                "spdx".to_string(),
            ),
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3054")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3054_spdx_labels_validation_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3055_valid_git_hash_labels_fixture() {
    let source = read_fixture("rules/DL3055.valid-git-hash-labels/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.revision".to_string(),
                "git-hash".to_string(),
            ),
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3055")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3055_valid_git_hash_labels_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3056_valid_semver_labels_fixture() {
    let source = read_fixture("rules/DL3056.valid-semver-labels/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.version".to_string(),
                "semver".to_string(),
            ),
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3056")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3056_valid_semver_labels_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3057_missing_healthcheck_fixture() {
    let source = read_fixture("rules/DL3057.missing-healthcheck/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        severity: BTreeMap::from([("DL3057".to_string(), Severity::Warning)]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3057")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3057_missing_healthcheck_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3058_valid_email_labels_fixture() {
    let source = read_fixture("rules/DL3058.valid-email-labels/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        label_schema: BTreeMap::from([
            (
                "org.opencontainers.image.authors".to_string(),
                "email".to_string(),
            ),
            (
                "org.opencontainers.image.title".to_string(),
                "text".to_string(),
            ),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3058")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3058_valid_email_labels_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3059_consecutive_run_fixture() {
    let source = read_fixture("rules/DL3059.consecutive-run/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3059")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3059_consecutive_run_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3060_yarn_cache_clean_fixture() {
    let source = read_fixture("rules/DL3060.yarn-cache-clean/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3060")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3060_yarn_cache_clean_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3061_instruction_order_fixture() {
    let source = read_fixture("rules/DL3061.instruction-order/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3061")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3061_instruction_order_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3062_pin_go_versions_fixture() {
    let source = read_fixture("rules/DL3062.pin-go-versions/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3062")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3062_pin_go_versions_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl3063_reserved_stage_name_fixture() {
    let source = read_fixture("rules/DL3063.reserved-stage-name/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL3063")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl3063_reserved_stage_name_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl4000_deprecated_maintainer_fixture() {
    let source = read_fixture("rules/DL4000.deprecated-maintainer/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let engine = RuleEngine::new(Profile::Default, Config::default());
    let findings = engine
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL4000")
        .collect::<Vec<_>>();
    let fixes = engine
        .fixes(&document)
        .into_iter()
        .filter(|fix| fix.title == "replace MAINTAINER with OCI authors label")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl4000_deprecated_maintainer_fixture",
        serde_json::json!({
            "findings": findings,
            "fixes": fixes,
        })
    );
}

#[test]
fn snapshots_dl4001_either_wget_or_curl_fixture() {
    let source = read_fixture("rules/DL4001.either-wget-or-curl/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL4001")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl4001_either_wget_or_curl_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl4003_cmd_cardinality_fixture() {
    let fixtures = ["DL4003.no-cmd", "DL4003.single-cmd", "DL4003.duplicate-cmd"];

    let cases = fixtures
        .into_iter()
        .map(|fixture| {
            let source = read_fixture(format!("rules/{fixture}/Dockerfile"));
            let document = parse_dockerfile(&source).expect("fixture should parse");
            let findings = RuleEngine::new(Profile::Default, Config::default())
                .lint(&document)
                .into_iter()
                .filter(|finding| finding.code == "DL4003")
                .collect::<Vec<_>>();

            serde_json::json!({
                "fixture": fixture,
                "findings": findings,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl4003_cmd_cardinality_fixture",
        serde_json::to_value(&cases).expect("cases should serialize")
    );
}

#[test]
fn snapshots_dl4004_entrypoint_cardinality_fixture() {
    let fixtures = [
        "DL4004.no-entrypoint",
        "DL4004.single-entrypoint",
        "DL4004.duplicate-entrypoint",
    ];

    let cases = fixtures
        .into_iter()
        .map(|fixture| {
            let source = read_fixture(format!("rules/{fixture}/Dockerfile"));
            let document = parse_dockerfile(&source).expect("fixture should parse");
            let findings = RuleEngine::new(Profile::Default, Config::default())
                .lint(&document)
                .into_iter()
                .filter(|finding| finding.code == "DL4004")
                .collect::<Vec<_>>();

            serde_json::json!({
                "fixture": fixture,
                "findings": findings,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl4004_entrypoint_cardinality_fixture",
        serde_json::to_value(&cases).expect("cases should serialize")
    );
}

#[test]
fn snapshots_dl4005_use_shell_for_default_shell_fixture() {
    let source = read_fixture("rules/DL4005.use-shell-for-default-shell/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL4005")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl4005_use_shell_for_default_shell_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_dl4006_pipefail_before_pipe_fixture() {
    let source = read_fixture("rules/DL4006.pipefail-before-pipe/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "DL4006")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "dl4006_pipefail_before_pipe_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdk1004_secret_mount_copied_to_layer_fixture() {
    let source = read_fixture("rules/RDK1004.secret-mount-copied-to-layer/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1004")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdk1004_secret_mount_copied_to_layer_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdk1005_ssh_mount_command_scope_fixture() {
    let source = read_fixture("rules/RDK1005.ssh-mount-command-scope/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1005")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdk1005_ssh_mount_command_scope_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdk1006_cache_mount_stable_id_fixture() {
    let source = read_fixture("rules/RDK1006.cache-mount-stable-id/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1006")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdk1006_cache_mount_stable_id_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdk1007_cache_mount_safe_sharing_fixture() {
    let source = read_fixture("rules/RDK1007.cache-mount-safe-sharing/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1007")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdk1007_cache_mount_safe_sharing_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdk1008_entitlement_opt_in_fixture() {
    let source = read_fixture("rules/RDK1008.entitlement-opt-in/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1008")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdk1008_entitlement_opt_in_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdk1008_entitlement_opt_in_config_fixture() {
    let source = read_fixture("rules/RDK1008.entitlement-opt-in/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let config = Config {
        allow_entitlements: BTreeSet::from([
            "network.host".to_string(),
            "security.insecure".to_string(),
        ]),
        ..Config::default()
    };
    let findings = RuleEngine::new(Profile::Default, config)
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1008")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdk1008_entitlement_opt_in_config_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdk1009_multi_platform_host_architecture_fixture() {
    let source = read_fixture("rules/RDK1009.multi-platform-host-architecture/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1009")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdk1009_multi_platform_host_architecture_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn snapshots_rdk1010_frontend_version_supports_syntax_fixture() {
    let source = read_fixture("rules/RDK1010.frontend-version-supports-syntax/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1010")
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "rdk1010_frontend_version_supports_syntax_fixture",
        serde_json::to_value(&findings).expect("findings should serialize")
    );
}

#[test]
fn rdk1010_allows_floating_frontend_tag() {
    let source = r#"# syntax=docker/dockerfile:1
FROM alpine:3.20
RUN --security=sandbox true
COPY --parents src/file /dst/
"#;
    let document = parse_dockerfile(source).expect("fixture should parse");
    let findings = RuleEngine::new(Profile::Default, Config::default())
        .lint(&document)
        .into_iter()
        .filter(|finding| finding.code == "RDK1010")
        .collect::<Vec<_>>();

    assert!(findings.is_empty());
}

#[test]
fn snapshots_rule_selection_matrix() {
    let source = read_fixture("rules/default-basic/Dockerfile");
    let document = parse_dockerfile(&source).expect("fixture should parse");

    let default_engine = RuleEngine::new(Profile::Default, Config::default());
    let compat_engine = RuleEngine::new(Profile::HadolintCompat, Config::default());

    let ignore_config = Config {
        ignore: BTreeSet::from(["DL3000".to_string()]),
        ..Config::default()
    };

    let severity_config = Config {
        severity: BTreeMap::from([("DL3007".to_string(), Severity::Error)]),
        ..Config::default()
    };

    let select_config = Config {
        select: BTreeSet::from(["RDK".to_string()]),
        ..Config::default()
    };

    let per_file_ignore_config = Config {
        per_file_ignores: BTreeMap::from([(
            "rules/**".to_string(),
            BTreeSet::from(["DL3000".to_string()]),
        )]),
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
            "ignore_dl3000": finding_codes(
                &RuleEngine::new(Profile::Default, ignore_config).lint(&document)
            ),
            "severity_override_dl3007": finding_codes(
                &RuleEngine::new(Profile::HadolintCompat, severity_config).lint(&document)
            ),
            "select_rdk": finding_codes(
                &RuleEngine::new(Profile::Default, select_config).lint(&document)
            ),
            "per_file_ignore_dl3000": finding_codes(
                &RuleEngine::new(Profile::Default, per_file_ignore_config)
                    .lint_path(std::path::Path::new("rules/default-basic/Dockerfile"), &document)
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
