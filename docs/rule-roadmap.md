# Rule Coverage Matrix

This matrix tracks the rule families `rudolint` covers for the v1 supported
surface and keeps future work separate from implemented rules.

`DL` compatibility IDs track Hadolint-derived behavior by fixtures and rule
documentation. `SC` IDs track ShellCheck-style rules for `RUN` commands. `RDK`
IDs track BuildKit-native behavior.

The `Enabled profiles` column lists where each implemented rule runs. The
`default` profile includes every implemented rule. The `hadolint-compat`
profile includes compatibility and shell rules, but excludes BuildKit-native
`RDK` rules. The `correctness`, `performance`, and `hardening` profiles narrow
the catalog to diagnostics with that signal.

## Implemented V1 Surface

Autofix statuses are limited to `safe`, `manual`, `not-applicable`, and
`not-yet`.

Negative fixture statuses are:

- `yes`: covered by a focused rule-specific negative fixture.
- `shared`: covered by the shared real-world noise corpus, which must remain
  finding-free across active profiles.
- `not-yet`: planned future coverage that is not implemented.

| Rule ID | Family | Enabled profiles | Default severity | Docs | Positive fixture | Negative fixture | Autofix | Source span |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `RDK1000` | BuildKit | `default`, `correctness` | `info` | yes | snapshot | shared | `safe` | yes |
| `RDK1001` | BuildKit | `default`, `hardening` | `warning` | yes | snapshot | shared | `not-applicable` | yes |
| `RDK1002` | BuildKit | `default`, `hardening` | `warning` | yes | snapshot | shared | `not-applicable` | yes |
| `RDK1003` | BuildKit | `default`, `performance` | `info` | yes | snapshot | shared | `not-applicable` | yes |
| `RDK1004` | BuildKit | `default`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `RDK1005` | BuildKit | `default`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `RDK1006` | BuildKit | `default`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `RDK1007` | BuildKit | `default`, `correctness`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `RDK1008` | BuildKit | `default`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `RDK1009` | BuildKit | `default`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `RDK1010` | BuildKit | `default`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `RUD1001` | Rudolint | `default` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3000` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3001` | Compatibility | `default`, `hadolint-compat`, `correctness` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3002` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | yes | `not-applicable` | yes |
| `DL3003` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3004` | Compatibility | `default`, `hadolint-compat`, `hardening` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3006` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3007` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3008` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3009` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3010` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3011` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3012` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | yes | `not-applicable` | yes |
| `DL3013` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3014` | Compatibility | `default`, `hadolint-compat`, `performance` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3015` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3016` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3018` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3019` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3020` | Compatibility | `default`, `hadolint-compat`, `correctness`, `hardening` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3021` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3022` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3023` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3024` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3025` | Compatibility | `default`, `hadolint-compat`, `correctness`, `hardening` | `warning` | yes | yes | shared | `manual` | yes |
| `DL3026` | Compatibility | `default`, `hadolint-compat`, `hardening` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3027` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3028` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3029` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3030` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3032` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3033` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3034` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3035` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3036` | Compatibility | `default`, `hadolint-compat`, `performance` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3037` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3038` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3040` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3041` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3042` | Compatibility | `default`, `hadolint-compat`, `performance` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3043` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3044` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3045` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3046` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3047` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3048` | Compatibility | `default`, `hadolint-compat`, `correctness` | `style` | yes | yes | shared | `not-applicable` | yes |
| `DL3049` | Compatibility | `default`, `hadolint-compat`, `correctness` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3050` | Compatibility | `default`, `hadolint-compat`, `correctness` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3051` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3052` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3053` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3054` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3055` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3056` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3057` | Compatibility | `default`, `hadolint-compat`, `correctness` | `ignore` | yes | yes | shared | `not-applicable` | yes |
| `DL3058` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3059` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3060` | Compatibility | `default`, `hadolint-compat`, `performance` | `info` | yes | yes | shared | `not-applicable` | yes |
| `DL3061` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `not-applicable` | yes |
| `DL3062` | Compatibility | `default`, `hadolint-compat`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL3063` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL4000` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | shared | `safe` | yes |
| `DL4001` | Compatibility | `default`, `hadolint-compat`, `correctness`, `hardening` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL4003` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | yes | `not-applicable` | yes |
| `DL4004` | Compatibility | `default`, `hadolint-compat`, `correctness` | `error` | yes | yes | yes | `not-applicable` | yes |
| `DL4005` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `DL4006` | Compatibility | `default`, `hadolint-compat`, `correctness` | `warning` | yes | yes | shared | `not-applicable` | yes |
| `SC2002` | Shell | `default`, `hadolint-compat`, `performance` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `SC2015` | Shell | `default`, `hadolint-compat`, `correctness` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `SC2046` | Shell | `default`, `hadolint-compat`, `correctness` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `SC2086` | Shell | `default`, `hadolint-compat`, `correctness` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `SC2155` | Shell | `default`, `hadolint-compat`, `correctness` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `SC2164` | Shell | `default`, `hadolint-compat`, `correctness` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `SC2181` | Shell | `default`, `hadolint-compat`, `correctness` | `warning` | yes | shared | shared | `not-applicable` | yes |

## Planned Future Shell Rules

Shell rules should come from the dedicated shell-analysis layer for `RUN`
commands. They should not be implemented by ad hoc substring checks.

| Rule ID | Family | Enabled profiles | Default severity | Docs | Positive fixture | Negative fixture | Autofix | Source span |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `SC1000` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1001` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1007` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1010` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1018` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1035` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1045` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1065` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1066` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1077` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1078` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1079` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1081` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1083` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1086` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC1095` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC2026` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC2035` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC2140` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC2154` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `SC2196` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |

## Planned Native And Compatibility Rules

No compatibility or BuildKit-native rules are currently planned in this roadmap.
New IDs should be added here only when they are intentionally tracked.
