ARG STAGE=prep
FROM alpine:3.21 AS base
FROM base AS prep-base
FROM base AS model-base
FROM ${STAGE}-base AS runtime
FROM $STAGE-base AS alternate
FROM alpine AS untagged
