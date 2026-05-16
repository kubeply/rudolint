# rudolint-fix

Owns autofix edit generation. Rules may propose fixes later, but source edits
and patch application should stay out of the rule implementations.

Fix behavior is part of rule completion. Each rule should eventually expose one
of:

- safe automatic fix.
- manual fix suggestion.
- no-fix rationale.
