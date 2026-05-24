# BuildKit SSH Mount

This fixture represents private source checkout with a BuildKit SSH mount.

It exists to protect SSH mount handling for commands that need an agent socket
only during a single `RUN` instruction.
