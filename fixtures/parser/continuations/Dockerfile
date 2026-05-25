FROM alpine:3.20
RUN apk add \
    curl \
    git
RUN apt-get update \
    && apt-get install -y --no-install-recommends unzip \
    # Comment-only lines do not end continued instructions.
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*
