FROM alpine:3.20 AS regular-apk
RUN apk add make build-base

FROM alpine:3.20 AS no-cache-apk
RUN apk add --no-cache make build-base

FROM debian:bookworm-slim AS apt
RUN apt-get update && apt-get install -y curl

FROM debian:bookworm-slim AS apt-with-unrelated-no-cache
RUN apt-get update && apt-get install -y --no-cache-placeholder curl
