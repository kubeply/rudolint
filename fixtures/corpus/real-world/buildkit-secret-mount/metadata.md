# BuildKit Secret Mount

This fixture represents package manager authentication with a BuildKit secret
mount.

It exists to protect secret-mount parsing and lint behavior for required
secrets, custom targets, and commands that consume secrets without copying them
into image layers.
