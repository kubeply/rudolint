# syntax=docker/dockerfile:1.7

FROM alpine:3.20

RUN --mount=type=ssh apk add git && git clone git@github.com:example/private.git /src
RUN --mount=type=ssh git clone git@github.com:example/private.git /src && go build ./...
RUN --mount=type=ssh sh -c 'git clone git@github.com:example/private.git /src'

RUN --mount=type=ssh git clone git@github.com:example/private.git /src
RUN --mount=type=ssh GIT_SSH_COMMAND="ssh -o StrictHostKeyChecking=yes" git fetch origin main
RUN apk add git && go version
