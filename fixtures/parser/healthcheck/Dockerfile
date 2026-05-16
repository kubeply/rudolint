FROM alpine:3.20
HEALTHCHECK --interval=30s --timeout=3s --retries=3 CMD wget -qO- http://localhost/health || exit 1
HEALTHCHECK NONE
