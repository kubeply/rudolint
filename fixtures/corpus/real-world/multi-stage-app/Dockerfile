# syntax=docker/dockerfile:1.7
FROM node:22.11.0-bookworm-slim AS deps

WORKDIR /app
COPY package.json package-lock.json ./
RUN --mount=type=cache,target=/root/.npm,id=npm-cache,sharing=locked \
    npm ci

FROM node:22.11.0-bookworm-slim AS build
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN npm run build

FROM gcr.io/distroless/nodejs22-debian12:nonroot
WORKDIR /app
COPY --from=build /app/dist ./dist
USER nonroot
CMD ["dist/server.js"]
