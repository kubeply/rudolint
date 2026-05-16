ARG TARGETPLATFORM=linux/amd64
ARG BUILDPLATFORM=linux/arm64
ARG TARGETARCH=amd64
ARG TARGETOS=linux
FROM --platform=$BUILDPLATFORM alpine:3.20 AS build
FROM --platform=$TARGETPLATFORM scratch@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
