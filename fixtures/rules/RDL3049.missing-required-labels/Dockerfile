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

FROM alpine:3.20
LABEL org.opencontainers.image.title="unnamed"
LABEL org.opencontainers.image.source="https://example.com/unnamed"

FROM alpine:3.20
LABEL org.opencontainers.image.title="fresh unnamed"

FROM alpine:3.20 AS numeric-source
RUN echo build

FROM scratch AS numeric-consumer
COPY --from=8 /out /out
LABEL org.opencontainers.image.title="numeric consumer"
LABEL org.opencontainers.image.source="https://example.com/numeric-consumer"

FROM alpine:3.20 AS CaseSource
RUN echo build

FROM scratch AS case-consumer
COPY --from=casesource /out /out
LABEL org.opencontainers.image.title="case consumer"
LABEL org.opencontainers.image.source="https://example.com/case-consumer"

FROM scratch AS external-copy
COPY --from=ghcr.io/example/image:latest /out /out
LABEL org.opencontainers.image.title="external copy"
LABEL org.opencontainers.image.source="https://example.com/external-copy"
