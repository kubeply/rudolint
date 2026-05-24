# Rule Coverage Matrix

This matrix tracks the rule families `rudolint` covers for the v1 supported
surface and keeps future work separate from implemented rules.

`RDL` compatibility IDs track Hadolint-derived behavior by fixtures and rule
documentation. `RSC` IDs track shell-analysis rules for `RUN` commands. `RDK`
IDs track BuildKit-native behavior.

## Implemented V1 Surface

| Rule ID | Family | Coverage |
| --- | --- | --- |
| `RDL1001` | Compatibility | Implemented |
| `RDL3000` | Compatibility | Implemented |
| `RDL3001` | Compatibility | Implemented |
| `RDL3002` | Compatibility | Implemented |
| `RDL3003` | Compatibility | Implemented |
| `RDL3004` | Compatibility | Implemented |
| `RDL3006` | Compatibility | Implemented |
| `RDL3007` | Compatibility | Implemented |
| `RDL3008` | Compatibility | Implemented |
| `RDL3009` | Compatibility | Implemented |
| `RDL3010` | Compatibility | Implemented |
| `RDL3011` | Compatibility | Implemented |
| `RDL3012` | Compatibility | Implemented |
| `RDL3013` | Compatibility | Implemented |
| `RDL3014` | Compatibility | Implemented |
| `RDL3015` | Compatibility | Implemented |
| `RDL3016` | Compatibility | Implemented |
| `RDL3018` | Compatibility | Implemented |
| `RDL3019` | Compatibility | Implemented |
| `RDL3020` | Compatibility | Implemented |
| `RDL3021` | Compatibility | Implemented |
| `RDL3022` | Compatibility | Implemented |
| `RDL3023` | Compatibility | Implemented |
| `RDL3024` | Compatibility | Implemented |
| `RDL3025` | Compatibility | Implemented |
| `RDL3026` | Compatibility | Implemented |
| `RDL3027` | Compatibility | Implemented |
| `RDL3028` | Compatibility | Implemented |
| `RDL3029` | Compatibility | Implemented |
| `RDL3030` | Compatibility | Implemented |
| `RDL3032` | Compatibility | Implemented |
| `RDL3033` | Compatibility | Implemented |
| `RDL3034` | Compatibility | Implemented |
| `RDL3035` | Compatibility | Implemented |
| `RDL3036` | Compatibility | Implemented |
| `RDL3037` | Compatibility | Implemented |
| `RDL3038` | Compatibility | Implemented |
| `RDL3040` | Compatibility | Implemented |
| `RDL3041` | Compatibility | Implemented |
| `RDL3042` | Compatibility | Implemented |
| `RDL3043` | Compatibility | Implemented |
| `RDL3044` | Compatibility | Implemented |
| `RDL3045` | Compatibility | Implemented |
| `RDL3046` | Compatibility | Implemented |
| `RDL3047` | Compatibility | Implemented |
| `RDL3048` | Compatibility | Implemented |
| `RDL3049` | Compatibility | Implemented |
| `RDL3050` | Compatibility | Implemented |
| `RDL3051` | Compatibility | Implemented |
| `RDL3052` | Compatibility | Implemented |
| `RDL3053` | Compatibility | Implemented |
| `RDL3054` | Compatibility | Implemented |
| `RDL3055` | Compatibility | Implemented |
| `RDL3056` | Compatibility | Implemented |
| `RDL3057` | Compatibility | Implemented |
| `RDL3058` | Compatibility | Implemented |
| `RDL3059` | Compatibility | Implemented |
| `RDL3060` | Compatibility | Implemented |
| `RDL3061` | Compatibility | Implemented |
| `RDL3062` | Compatibility | Implemented |
| `RDL3063` | Compatibility | Implemented |
| `RDL4000` | Compatibility | Implemented |
| `RDL4001` | Compatibility | Implemented |
| `RDL4003` | Compatibility | Implemented |
| `RDL4004` | Compatibility | Implemented |
| `RDL4005` | Compatibility | Implemented |
| `RDL4006` | Compatibility | Implemented |
| `RSC2002` | Shell | Implemented |
| `RSC2015` | Shell | Implemented |
| `RSC2046` | Shell | Implemented |
| `RSC2086` | Shell | Implemented |
| `RSC2155` | Shell | Implemented |
| `RSC2164` | Shell | Implemented |
| `RSC2181` | Shell | Implemented |
| `RDK1000` | BuildKit | BuildKit feature used without explicit frontend directive |
| `RDK1001` | BuildKit | Secret-like build input declared as `ARG` or `ENV` |
| `RDK1002` | BuildKit | Secret-like value passed through `RUN` without secret mount |
| `RDK1003` | BuildKit | Package install step without cache mount opportunity |
| `RDK1004` | BuildKit | Secret mount target copied into an image layer |
| `RDK1005` | BuildKit | SSH mount used without explicit command scoping |
| `RDK1006` | BuildKit | Cache mount missing stable `id` in multi-stage builds |
| `RDK1007` | BuildKit | Cache mount sharing mode unsafe for common package managers |
| `RDK1008` | BuildKit | BuildKit entitlement used without config opt-in |
| `RDK1009` | BuildKit | Multi-platform build uses host architecture accidentally |
| `RDK1010` | BuildKit | Frontend version too old for used syntax |

## Planned Future Shell Rules

Shell rules should come from the dedicated shell-analysis layer for `RUN`
commands. They should not be implemented by ad hoc substring checks.

| Rule ID | Family | Coverage |
| --- | --- | --- |
| `RSC1000` | Shell | Planned |
| `RSC1001` | Shell | Planned |
| `RSC1007` | Shell | Planned |
| `RSC1010` | Shell | Planned |
| `RSC1018` | Shell | Planned |
| `RSC1035` | Shell | Planned |
| `RSC1045` | Shell | Planned |
| `RSC1065` | Shell | Planned |
| `RSC1066` | Shell | Planned |
| `RSC1077` | Shell | Planned |
| `RSC1078` | Shell | Planned |
| `RSC1079` | Shell | Planned |
| `RSC1081` | Shell | Planned |
| `RSC1083` | Shell | Planned |
| `RSC1086` | Shell | Planned |
| `RSC1095` | Shell | Planned |
| `RSC2026` | Shell | Planned |
| `RSC2035` | Shell | Planned |
| `RSC2140` | Shell | Planned |
| `RSC2154` | Shell | Planned |
| `RSC2196` | Shell | Planned |

## Planned Native And Compatibility Rules

No compatibility or BuildKit-native rules are currently planned in this roadmap.
New IDs should be added here only when they are intentionally tracked.
