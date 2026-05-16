FROM fedora:40
RUN dnf install -y curl
RUN dnf install -y curl && dnf clean all
RUN dnf clean all
RUN dnf update -y curl
RUN echo dnf install curl
RUN cd /tmp && dnf install -y make
