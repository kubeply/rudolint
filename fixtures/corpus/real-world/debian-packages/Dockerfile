FROM debian:12.8-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates=20230311 \
        curl=7.88.1-10+deb12u8 \
        tini=0.19.0-1 \
    && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["bash"]
