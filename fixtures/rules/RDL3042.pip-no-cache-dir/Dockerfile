FROM python:3.12-slim AS base
RUN pip install flask
RUN pip install --no-cache-dir flask
RUN PIP_NO_CACHE_DIR=1 pip install click
RUN python -m pip install build
RUN python -m pip install --no-cache-dir wheel
RUN --mount=type=cache,target=/root/.cache/pip pip install pytest
ENV PIP_NO_CACHE_DIR=true
RUN pip install requests

FROM base AS inherited
RUN pip install rich

FROM python:3.12-slim AS fresh
RUN pip3 install httpx
ENV PIP_NO_CACHE_DIR=0
RUN pip install pydantic
