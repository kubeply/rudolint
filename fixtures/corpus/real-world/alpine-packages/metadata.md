# Alpine Package Installs

This fixture represents small runtime images that install a few Alpine packages
with `apk add --no-cache`.

It exists to protect package-install rule behavior on common Alpine syntax,
including multiline package lists, pinned package versions, and JSON-form
entrypoints.
