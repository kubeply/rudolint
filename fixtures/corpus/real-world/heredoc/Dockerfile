# syntax=docker/dockerfile:1.7
FROM busybox:1.36.1

RUN <<'EOF'
set -eu
mkdir -p /usr/local/bin
mkdir -p /etc/app/config
cat > /usr/local/bin/healthcheck <<'SCRIPT'
#!/bin/sh
test -d /etc/app/config
SCRIPT
chmod +x /usr/local/bin/healthcheck
EOF

HEALTHCHECK CMD ["/usr/local/bin/healthcheck"]
