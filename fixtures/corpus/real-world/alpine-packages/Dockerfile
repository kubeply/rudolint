FROM alpine:3.20

RUN apk add --no-cache \
    ca-certificates=20240705-r0 \
    curl=8.12.1-r0 \
    tini=0.19.0-r3

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["sh"]
