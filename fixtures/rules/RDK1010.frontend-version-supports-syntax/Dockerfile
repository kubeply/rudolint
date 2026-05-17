# syntax=docker/dockerfile:1.3

FROM alpine:3.20

RUN --mount=type=cache,target=/var/cache/apk apk add curl
RUN --network=none true
RUN --security=sandbox true
RUN <<EOF
echo hello
EOF

COPY --link src/ /dst/
COPY --chmod=+x src/tool /usr/local/bin/tool
COPY --parents src/file /dst/
ADD --checksum=sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa https://example.invalid/archive.tar.gz /tmp/archive.tar.gz
