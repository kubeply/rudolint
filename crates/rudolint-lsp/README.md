# rudolint-lsp

Owns the stdio language server and editor integration primitives. Editor
protocol concerns should not live in the CLI, parser, or rule crates.

## Run

From the repository:

```bash
cargo run -p rudolint-lsp --bin rudolint-lsp
```

Or install the server binary while developing:

```bash
cargo install --path crates/rudolint-lsp
```

Then configure an editor client to launch:

```bash
rudolint-lsp
```

The server speaks LSP over stdio. It does not print human-readable output on
stdout.

## Capabilities

- full document synchronization
- diagnostics on open and full-document changes
- hover content for rule codes such as `RDL3007` and `RDK1004`
- safe quick-fix code actions from automatically applicable fixes
- workspace and document-level `.rudolint.yaml` discovery

## Editor Wrappers

The server is usable by any editor that can launch a custom stdio LSP command.
Dedicated wrappers should stay thin: register Dockerfile buffers, launch
`rudolint-lsp`, and pass the workspace folder. VS Code, Neovim, and Zed wrappers
can be built on top of this binary without moving lint behavior into the
wrapper.
