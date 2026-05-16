FROM alpine:3.20
HEALTHCHECK CMD wget -qO- http://localhost/health || exit 1
HEALTHCHECK NONE
HEALTHCHECK CMD curl -f http://localhost/health
