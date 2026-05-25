FROM fedora:40
RUN dnf install -y curl
RUN dnf install -y curl && dnf clean all
RUN dnf clean all
RUN dnf update -y curl
RUN echo dnf install curl
RUN cd /tmp && dnf install -y make
RUN dnf clean all && dnf install -y git
RUN dnf install -y clean all
RUN dnf install -y vim && dnf clean all && dnf install -y jq
