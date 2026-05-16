FROM opensuse/leap:15.6
RUN zypper install -y curl
RUN zypper install -y curl=8.6.0
RUN zypper install -y ca-certificates
RUN zypper install -y python3-devel-3.11.10
RUN zypper install -y curl=8.6.0 git
RUN zypper update -y curl
RUN echo zypper install curl
RUN cd /tmp && zypper install -y make
