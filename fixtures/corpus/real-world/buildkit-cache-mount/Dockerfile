# syntax=docker/dockerfile:1.7
FROM python:3.12.7-slim AS wheels

WORKDIR /src
COPY requirements.txt .
RUN --mount=type=cache,target=/root/.cache/pip,id=pip-cache,sharing=locked \
    pip wheel --wheel-dir /wheels -r requirements.txt
