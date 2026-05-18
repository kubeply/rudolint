# syntax=docker/dockerfile:1.20
FROM --platform=$BUILDPLATFORM alpine:3.20 AS deps
RUN --mount=type=cache,target=/var/cache/apk,sharing=locked apk add --no-cache git openssh-client curl
RUN --mount=type=secret,id=npmrc,target=/run/secrets/npmrc cp /run/secrets/npmrc /tmp/npmrc
RUN --mount=type=ssh sh -c 'git clone git@github.com:example/private.git /src && cd /src && make'

FROM --platform=$BUILDPLATFORM golang:1.22-alpine AS build
ARG TARGETPLATFORM
ARG TARGETARCH
WORKDIR /src
COPY --from=deps /src /src
RUN --mount=type=cache,target=/root/.cache/go-build,sharing=locked go env GOARCH && go build ./...

FROM alpine:3.20
RUN --network=host apk add --no-cache ca-certificates
RUN --security=insecure echo insecure
COPY --from=build /src/bin/app /usr/local/bin/app
USER 1000
ENTRYPOINT ["/usr/local/bin/app"]
