# syntax=docker/dockerfile:1.7
FROM alpine/git:2.45.2 AS source

WORKDIR /src
RUN --mount=type=ssh \
    git clone git@github.com:example/private-module.git module
