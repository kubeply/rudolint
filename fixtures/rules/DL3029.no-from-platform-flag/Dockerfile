FROM alpine:3.20 AS base
FROM --platform=linux/amd64 alpine:3.20 AS amd64
FROM --platform=$BUILDPLATFORM golang:1.22 AS build
COPY --from=base /etc/os-release /os-release
