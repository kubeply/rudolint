# syntax=docker/dockerfile:1.7
FROM alpine:latest AS base
WORKDIR app
ARG API_TOKEN
RUN apk add curl
ADD ./app /app
CMD echo hello
CMD ["echo", "world"]
