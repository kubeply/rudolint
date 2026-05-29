FROM ubuntu:24.04
RUN apt-get update
RUN apt-get install -y curl
RUN apt-get install -y curl=8.5.0-2ubuntu10.6
RUN apt-get install --no-install-recommends curl=8.5.0-2ubuntu10.6 ca-certificates
RUN apt-get install -t noble curl=8.5.0-2ubuntu10.6
RUN apt-get install --target-release noble curl=8.5.0-2ubuntu10.6
RUN apt-get install -c /tmp/apt.conf curl=8.5.0-2ubuntu10.6
RUN echo apt-get install curl
RUN cd /tmp && apt-get install -y git=1:2.43.0-1ubuntu7.2 make
RUN apt-get update && \
    apt-get install --no-install-recommends -y \
        ca-certificates=20211016 \
        curl='7.81.0-*' \
        locales='2.35-*' \
    ##
    # comment-only continuation lines should not become package arguments.
        bash='5.1-*' \
        git='1:2.34.1-*' \
        wget='1.21.2-*' \
    && rm -rf /var/lib/apt/lists/*
