# syntax=docker/dockerfile:1.7

FROM rust:1.95-slim-bookworm@sha256:d7482085ff5b415f84dba5647ae71606650bdef00db7aeb69f4b3d170c3e4082 AS build

ARG TARGETARCH

WORKDIR /src

# Builder-only packages come from a digest-pinned Rust image. Pinning Debian
# revisions here would make routine security updates brittle without changing
# the scratch runtime image.
# rudolint ignore=DL3008
RUN --mount=type=cache,id=rudolint-apt-cache,target=/var/cache/apt,sharing=locked \
    apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates musl-tools \
    && rm -rf /var/lib/apt/lists/*

RUN case "$TARGETARCH" in \
        amd64) echo "x86_64-unknown-linux-musl" > /tmp/rust-target ;; \
        arm64) echo "aarch64-unknown-linux-musl" > /tmp/rust-target ;; \
        *) echo "unsupported target architecture: $TARGETARCH" >&2; exit 2 ;; \
    esac \
    && rustup target add "$(cat /tmp/rust-target)"

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY schemas ./schemas

RUN --mount=type=cache,id=rudolint-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=rudolint-cargo-target,target=/src/target \
    target="$(cat /tmp/rust-target)" \
    && cargo build --locked --release -p rudolint --target "$target" \
    && cp "target/$target/release/rudolint" /usr/local/bin/rudolint

FROM scratch

ARG RUDOLINT_VERSION=dev
ARG VCS_REF=unknown

LABEL org.opencontainers.image.title="rudolint" \
      org.opencontainers.image.description="BuildKit-native Dockerfile linter" \
      org.opencontainers.image.source="https://github.com/kubeply/rudolint" \
      org.opencontainers.image.version="$RUDOLINT_VERSION" \
      org.opencontainers.image.revision="$VCS_REF" \
      org.opencontainers.image.licenses="Apache-2.0"

COPY --from=build /usr/local/bin/rudolint /usr/local/bin/rudolint

USER 65532:65532
WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/rudolint"]
