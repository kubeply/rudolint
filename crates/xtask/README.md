# xtask

Repository maintenance commands that should not ship in the `rudolint` runtime
binary.

Current commands:

```bash
cargo run -p xtask -- update-oracle hadolint
```

The oracle update command records the installed Hadolint binary and version in
`fixtures/compat/oracles/hadolint.json`. Set `RUDOLINT_ORACLE_BIN` to use a
specific binary.
