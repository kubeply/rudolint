# syntax=docker/dockerfile:1.7
FROM --platform=$BUILDPLATFORM alpine:3.20 AS deps
RUN --mount=type=cache,target=/var/cache/apk apk add curl git

FROM deps AS build
WORKDIR /src
COPY . /src
RUN <<EOF
echo building
EOF

FROM alpine:3.20
WORKDIR /app
COPY --from=build /src/bin/app /app/app
USER 1000
ENTRYPOINT ["/app/app"]
