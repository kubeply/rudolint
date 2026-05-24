FROM alpine:3.20

WORKDIR /app
COPY --chown=65532:65532 app /usr/local/bin/app
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/app"]
