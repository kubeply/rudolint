# Clean Runtime Noise

This fixture represents a compact runtime image that already follows the common
default-profile recommendations.

It exists to catch broad false positives on absolute `WORKDIR`, JSON-form
entrypoints, non-root users, and ownership-aware copies.
