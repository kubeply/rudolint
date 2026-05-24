# Debian Package Installs

This fixture represents Debian or Ubuntu style package installation with
`apt-get update`, `apt-get install --no-install-recommends`, and apt list
cleanup in the same layer.

It exists to protect package-install rule behavior for multiline apt workflows
and pinned package versions.
