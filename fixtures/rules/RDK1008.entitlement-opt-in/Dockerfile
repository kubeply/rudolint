# syntax=docker/dockerfile:1.7

FROM alpine:3.20

RUN --network=host apk add curl
RUN --security=insecure mount -t proc proc /proc
RUN --network=host --security=insecure apk add git

RUN --network=none apk add curl
RUN --security=sandbox true
