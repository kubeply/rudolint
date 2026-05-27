FROM alpine:3.20 AS base
FROM --platform=linux/amd64 alpine:3.20 AS amd64
FROM --platform=$BUILDPLATFORM golang:1.22 AS build
FROM --platform=${TARGETPLATFORM} alpine:3.20 AS target
FROM --platform=$CUSTOM_BUILD_PLATFORM alpine:3.20 AS custom
FROM --platform=linux/${TARGETARCH} alpine:3.20 AS target_arch
COPY --from=base /etc/os-release /os-release
