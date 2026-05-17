# syntax=docker/dockerfile:1.7

FROM alpine:3.20

RUN --mount=type=secret,id=api_token cp /run/secrets/api_token /app/token
RUN --mount=type=secret,id=deploy_key,target=/tmp/deploy_key install -m 0600 /tmp/deploy_key /root/.ssh/id_rsa
RUN --mount=type=secret,id=credentials,target=/run/credentials rsync /run/credentials/service.json /app/service.json

RUN --mount=type=secret,id=api_token tool --token-file /run/secrets/api_token
RUN --mount=type=secret,id=api_token cp /tmp/source /app/source
RUN cp /run/secrets/api_token /app/token
