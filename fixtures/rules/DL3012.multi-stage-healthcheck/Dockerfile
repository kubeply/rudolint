FROM alpine:3.20 AS base
HEALTHCHECK CMD wget -qO- http://localhost/health || exit 1

FROM base AS runtime
HEALTHCHECK CMD wget -qO- http://localhost/ready || exit 1

