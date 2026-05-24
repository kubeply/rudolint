# syntax=docker/dockerfile:1.7
FROM node:22.11.0-bookworm-slim AS deps

WORKDIR /app
COPY package.json package-lock.json ./
RUN --mount=type=secret,id=npmrc,target=/root/.npmrc,required=true \
    npm ci --ignore-scripts
