# syntax=docker/dockerfile:1.7
FROM --platform=$BUILDPLATFORM alpine:3.20 AS base
RUN --mount=type=cache,target=/var/cache/apk apk add curl
RUN <<EOF
echo hello
EOF
COPY --from=base /src /dst
