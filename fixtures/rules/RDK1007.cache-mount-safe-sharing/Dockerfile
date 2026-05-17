# syntax=docker/dockerfile:1.7

FROM debian:12

RUN --mount=type=cache,target=/var/cache/apt apt-get update && apt-get install -y curl
RUN --mount=type=cache,target=/var/cache/dnf,sharing=shared dnf install -y git

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked apt-get update && apt-get install -y curl
RUN --mount=type=cache,target=/root/.cache/go-build apt-get update && go build ./...
RUN --mount=type=cache,target=/root/.npm npm ci
RUN --mount=type=cache,target=/var/cache/apk apk add curl
