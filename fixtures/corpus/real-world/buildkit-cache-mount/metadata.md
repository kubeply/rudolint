# BuildKit Cache Mount

This fixture represents dependency preparation steps that use a BuildKit cache
mount to avoid repeating expensive downloads.

It exists to protect parser and rule behavior for `RUN --mount=type=cache`,
including stable cache identifiers and sharing options.
