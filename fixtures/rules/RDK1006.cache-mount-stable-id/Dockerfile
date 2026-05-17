# syntax=docker/dockerfile:1.7

FROM alpine:3.20 AS deps
RUN --mount=type=cache,target=/var/cache/apk apk add build-base
RUN --mount=type=cache,id=apk-cache,target=/var/cache/apk apk add curl

FROM deps AS build
RUN --mount=type=cache,target=/root/.cache/go-build go build ./...
RUN --mount=type=cache,id=go-build,target=/root/.cache/go-build go test ./...

FROM scratch
COPY --from=build /out/app /app
