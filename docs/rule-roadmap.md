# Rule Coverage Matrix

This matrix tracks the rule families `rudolint` covers for the v1 supported
surface and keeps future work separate from implemented rules.

`RDL` compatibility IDs track Hadolint-derived behavior by fixtures and rule
documentation. `RSC` IDs track shell-analysis rules for `RUN` commands. `RDK`
IDs track BuildKit-native behavior.

## Implemented V1 Surface

Autofix statuses are limited to `safe`, `manual`, `not-applicable`, and
`not-yet`.

| Rule ID | Family | Profile | Default severity | Docs | Positive fixture | Negative fixture | Autofix | Source span |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `RDK1000` | BuildKit | `default` | `info` | yes | snapshot | not audited | `safe` | yes |
| `RDK1001` | BuildKit | `default` | `warning` | yes | snapshot | not audited | `not-applicable` | yes |
| `RDK1002` | BuildKit | `default` | `warning` | yes | snapshot | not audited | `not-applicable` | yes |
| `RDK1003` | BuildKit | `default` | `info` | yes | snapshot | not audited | `not-applicable` | yes |
| `RDK1004` | BuildKit | `default` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDK1005` | BuildKit | `default` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDK1006` | BuildKit | `default` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDK1007` | BuildKit | `default` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDK1008` | BuildKit | `default` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDK1009` | BuildKit | `default` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDK1010` | BuildKit | `default` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL1001` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3000` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3001` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3002` | Compatibility | `hadolint-compat` | `warning` | yes | yes | yes | `not-applicable` | yes |
| `RDL3003` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3004` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3006` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3007` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3008` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3009` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3010` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3011` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3012` | Compatibility | `hadolint-compat` | `error` | yes | yes | yes | `not-applicable` | yes |
| `RDL3013` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3014` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3015` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3016` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3018` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3019` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3020` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3021` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3022` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3023` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3024` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3025` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `manual` | yes |
| `RDL3026` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3027` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3028` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3029` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3030` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3032` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3033` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3034` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3035` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3036` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3037` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3038` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3040` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3041` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3042` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3043` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3044` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3045` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3046` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3047` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3048` | Compatibility | `hadolint-compat` | `style` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3049` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3050` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3051` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3052` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3053` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3054` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3055` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3056` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3057` | Compatibility | `hadolint-compat` | `ignore` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3058` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3059` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3060` | Compatibility | `hadolint-compat` | `info` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3061` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3062` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL3063` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL4000` | Compatibility | `hadolint-compat` | `error` | yes | yes | not audited | `safe` | yes |
| `RDL4001` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL4003` | Compatibility | `hadolint-compat` | `warning` | yes | yes | yes | `not-applicable` | yes |
| `RDL4004` | Compatibility | `hadolint-compat` | `error` | yes | yes | yes | `not-applicable` | yes |
| `RDL4005` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RDL4006` | Compatibility | `hadolint-compat` | `warning` | yes | yes | not audited | `not-applicable` | yes |
| `RSC2002` | Shell | `hadolint-compat` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `RSC2015` | Shell | `hadolint-compat` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `RSC2046` | Shell | `hadolint-compat` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `RSC2086` | Shell | `hadolint-compat` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `RSC2155` | Shell | `hadolint-compat` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `RSC2164` | Shell | `hadolint-compat` | `warning` | yes | shared | shared | `not-applicable` | yes |
| `RSC2181` | Shell | `hadolint-compat` | `warning` | yes | shared | shared | `not-applicable` | yes |

## Planned Future Shell Rules

Shell rules should come from the dedicated shell-analysis layer for `RUN`
commands. They should not be implemented by ad hoc substring checks.

| Rule ID | Family | Profile | Default severity | Docs | Positive fixture | Negative fixture | Autofix | Source span |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `RSC1000` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1001` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1007` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1010` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1018` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1035` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1045` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1065` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1066` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1077` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1078` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1079` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1081` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1083` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1086` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC1095` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC2026` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC2035` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC2140` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC2154` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |
| `RSC2196` | Shell | future | n/a | planned | not-yet | not-yet | not-yet | not-yet |

## Planned Native And Compatibility Rules

No compatibility or BuildKit-native rules are currently planned in this roadmap.
New IDs should be added here only when they are intentionally tracked.
