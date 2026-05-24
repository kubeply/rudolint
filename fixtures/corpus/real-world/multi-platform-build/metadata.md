# Multi-Platform Build

This fixture represents cross-platform builds that use BuildKit platform
arguments and `FROM --platform=$BUILDPLATFORM`.

It exists to protect parser and rule behavior for `TARGETOS`, `TARGETARCH`,
cache mounts keyed by architecture, and multi-stage static runtime images.
