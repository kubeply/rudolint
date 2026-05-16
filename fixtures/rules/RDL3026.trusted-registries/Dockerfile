FROM ghcr.io/example/build:1.0 AS build
RUN mkdir -p /out && touch /out/app

FROM docker.io/library/alpine:3.20 AS untrusted
COPY --from=build /out/app /app

FROM localhost:5000/example/runtime:1.0 AS local
COPY --from=untrusted /app /app

FROM alpine:3.20 AS implicit
COPY --from=local /app /app

FROM scratch AS empty
COPY --from=implicit /app /app

FROM build AS reused
COPY --from=empty /app /app
