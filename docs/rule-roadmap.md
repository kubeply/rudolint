# Rule Roadmap

This roadmap tracks the rule families `rudolint` intends to cover.

`RDL` compatibility IDs track Hadolint-derived behavior by fixtures and rule
documentation. Descriptions in this file are intentionally short and
project-local.

## Implemented Compatibility Rules

- `RDL1001`
- `RDL3000`
- `RDL3001`
- `RDL3002`
- `RDL3003`
- `RDL3004`
- `RDL3006`
- `RDL3007`
- `RDL3008`
- `RDL3009`
- `RDL3010`
- `RDL3011`
- `RDL3012`
- `RDL3013`
- `RDL3014`
- `RDL3015`
- `RDL3016`
- `RDL3018`
- `RDL3019`
- `RDL3020`
- `RDL3021`
- `RDL3022`
- `RDL3023`
- `RDL3024`
- `RDL3025`
- `RDL3026`
- `RDL3027`
- `RDL3028`
- `RDL3029`
- `RDL3030`
- `RDL3032`
- `RDL3033`
- `RDL3034`
- `RDL3035`
- `RDL3036`
- `RDL3037`
- `RDL3038`
- `RDL3040`
- `RDL3041`
- `RDL3042`
- `RDL3043`
- `RDL3044`
- `RDL3045`
- `RDL3046`
- `RDL3047`
- `RDL3048`
- `RDL3049`
- `RDL3050`
- `RDL3051`
- `RDL3052`
- `RDL3053`
- `RDL3054`
- `RDL3055`
- `RDL3056`
- `RDL3057`
- `RDL3058`
- `RDL3059`
- `RDL3060`
- `RDL3061`
- `RDL3062`
- `RDL3063`
- `RDL4000`
- `RDL4001`
- `RDL4003`
- `RDL4004`
- `RDL4005`
- `RDL4006`

## Planned Compatibility Rules

No compatibility rules are currently planned in this roadmap. New compatibility
IDs should be added here when they are intentionally tracked.

## Implemented Shell Rules

`RSC` rules should come from a dedicated shell-analysis layer for `RUN`
commands. They should not be implemented by ad hoc substring checks.

- `RSC2002`
- `RSC2015`
- `RSC2046`
- `RSC2086`
- `RSC2155`
- `RSC2164`
- `RSC2181`

## Planned Shell Rules

Initial tracked IDs:

- `RSC1000`
- `RSC1001`
- `RSC1007`
- `RSC1010`
- `RSC1018`
- `RSC1035`
- `RSC1045`
- `RSC1065`
- `RSC1066`
- `RSC1077`
- `RSC1078`
- `RSC1079`
- `RSC1081`
- `RSC1083`
- `RSC1086`
- `RSC1095`
- `RSC2026`
- `RSC2035`
- `RSC2140`
- `RSC2154`
- `RSC2196`

## BuildKit-Native Rules

Implemented:

- `RDK1000`: BuildKit feature used without explicit frontend directive.
- `RDK1001`: Secret-like build input declared as `ARG` or `ENV`.
- `RDK1002`: Secret-like value passed through `RUN` without secret mount.
- `RDK1003`: Package install step without cache mount opportunity.
- `RDK1004`: Secret mount target copied into an image layer.
- `RDK1005`: SSH mount used without explicit command scoping.
- `RDK1006`: Cache mount missing stable `id` in multi-stage builds.
- `RDK1007`: Cache mount sharing mode unsafe for common package managers.
- `RDK1008`: BuildKit network/security entitlement used without config opt-in.
- `RDK1009`: Multi-platform build uses host architecture accidentally.
- `RDK1010`: Frontend version too old for used syntax.

Planned:

No BuildKit-native rules are currently planned in this roadmap. New BuildKit
IDs should be added here when they are intentionally tracked.
