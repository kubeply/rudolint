FROM cgr.dev/chainguard/static:1.0.0

ARG BUILD_DATE=2026-01-01T00:00:00Z
ARG VCS_REF=0000000000000000000000000000000000000000
ARG VERSION=1.0.0
LABEL org.opencontainers.image.created=$BUILD_DATE \
      org.opencontainers.image.revision=$VCS_REF \
      org.opencontainers.image.version=$VERSION \
      org.opencontainers.image.source="https://github.com/kubeply/rudolint" \
      org.opencontainers.image.licenses="Apache-2.0"

USER nonroot
ENTRYPOINT ["/app"]
