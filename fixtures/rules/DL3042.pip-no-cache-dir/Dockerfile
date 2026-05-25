FROM python:3.12-slim AS base
RUN pip install flask
RUN pip install --no-cache-dir flask
RUN PIP_NO_CACHE_DIR=1 pip install click
RUN python -m pip install build
RUN python -m pip install --no-cache-dir wheel
RUN --mount=type=cache,target=/root/.cache/pip pip install pytest
ENV PIP_NO_CACHE_DIR=true
RUN pip install requests
RUN PIP_NO_CACHE_DIR=0 pip install chardet
ENV PIP_NO_CACHE_DIR=0
RUN pip install urllib3

FROM base AS inherited
RUN pip install rich

FROM python:3.12-slim AS fresh
RUN pip3 install httpx
ENV PIP_NO_CACHE_DIR=0
RUN pip install pydantic
RUN PIP_NO_CACHE_DIR=1 pip install anyio && pip install h11

FROM python:3.12-slim
ENV PIP_NO_CACHE_DIR=true
RUN pip install markupsafe

FROM python:3.12-slim
RUN pip install idna
RUN pip install --config-settings PIP_NO_CACHE_DIR=1 attrs

FROM 3 AS from_index
RUN pip install six
