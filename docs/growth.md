# Growth Backlog

This backlog tracks practical post-v1 work that can make `rudolint` more useful
and easier to adopt.

## Priorities

1. Expand ShellCheck-style `SC` coverage. The current planned rule surface is
   entirely shell-focused, so this is the clearest rule-coverage improvement.
2. Build a real-world adoption corpus from popular repositories with deep
   Dockerfiles. Track false positives, ignored Hadolint rules, missing rules,
   and migration friction.
3. Improve autofix coverage for rules where fixes are deterministic or useful
   as manual suggestions.
4. Make the LSP easier to install in editors with thin VS Code, Neovim, or Zed
   wrappers that launch the existing `rudolint-lsp` binary.
5. Add config discovery and explanation tooling, such as a command that shows
   the selected config file, profile, ignored rules, and severity overrides.
6. Add repository-wide semantic rules for cases that need project context, such
   as copied files, `.dockerignore`, lockfiles, or base-image consistency.
7. Improve GitHub Action UX with clearer job summaries or annotations in
   addition to SARIF.
8. Evaluate whether the signal profiles should eventually replace the reserved
   `strict` profile concept.
9. Keep performance reporting readable by aligning benchmark names with user
   scenarios and refreshing public charts.
10. Reduce remaining release and Marketplace manual steps.

## Near-Term Focus

The strongest next sequence is:

1. real-world adoption corpus
2. planned `SC` rule coverage
3. autofix coverage

That sequence should improve correctness and user value without destabilizing
the crate architecture.
