# Multi-Stage Application

This fixture represents a typical application image with separate dependency,
build, and runtime stages.

It exists to protect parser and rule behavior for cross-stage copies, BuildKit
cache mounts, non-root runtime images, and JSON-form commands.
