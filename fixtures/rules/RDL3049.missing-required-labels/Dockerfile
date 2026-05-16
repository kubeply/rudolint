FROM alpine:3.20 AS builder
RUN echo build

FROM scratch AS complete
LABEL org.opencontainers.image.title="rudolint"
LABEL org.opencontainers.image.source="https://github.com/kubeply/rudolint"

FROM alpine:3.20 AS partial
LABEL org.opencontainers.image.title="rudolint"

FROM alpine:3.20 AS inherited
LABEL org.opencontainers.image.title="base"
LABEL org.opencontainers.image.source="https://example.com/base"

FROM inherited AS child

FROM alpine:3.20 AS consumer
COPY --from=builder /out /out
LABEL org.opencontainers.image.title="consumer"
LABEL org.opencontainers.image.source="https://example.com/consumer"
