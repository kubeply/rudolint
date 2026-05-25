FROM fedora:40
RUN dnf install -y curl
RUN dnf install -y curl-8.6.0
RUN dnf install -y ca-certificates
RUN dnf install -y python3-devel-3.12.4
RUN dnf install -y curl-8.6.0 git
RUN dnf update -y curl
RUN echo dnf install curl
RUN cd /tmp && dnf install -y make
RUN dnf -y install jq
RUN dnf install -y --exclude kernel tar-1.35
