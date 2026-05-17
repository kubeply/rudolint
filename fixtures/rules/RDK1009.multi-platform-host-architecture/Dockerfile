# syntax=docker/dockerfile:1.7

FROM --platform=$BUILDPLATFORM alpine:3.20 AS build
ARG TARGETARCH
RUN apk add --no-cache curl
RUN curl -fsSL "https://example.invalid/tool-$(uname -m).tar.gz" -o /tmp/tool.tar.gz
RUN arch > /tmp/build-arch
RUN curl -fsSL "https://example.invalid/tool-${TARGETARCH}.tar.gz" -o /tmp/tool.tar.gz

FROM --platform=$TARGETPLATFORM alpine:3.20 AS target
RUN TARGETARCH=${TARGETARCH} echo "building for $TARGETARCH"

FROM --platform=$BUILDPLATFORM scratch
