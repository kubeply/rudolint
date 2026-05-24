# syntax=docker/dockerfile:1.7
ARG GO_VERSION=1.23.3

FROM --platform=$BUILDPLATFORM golang:${GO_VERSION}-bookworm AS build
ARG TARGETOS
ARG TARGETARCH
WORKDIR /src
COPY go.mod go.sum ./
RUN --mount=type=cache,target=/go/pkg/mod,id=go-mod,sharing=locked \
    go mod download
COPY . .
RUN --mount=type=cache,target=/root/.cache/go-build,id=go-build-${TARGETARCH},sharing=locked \
    GOOS=$TARGETOS GOARCH=$TARGETARCH go build -o /out/app ./cmd/app

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /out/app /usr/local/bin/app
USER nonroot
ENTRYPOINT ["/usr/local/bin/app"]
