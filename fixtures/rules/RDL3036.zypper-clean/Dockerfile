FROM opensuse/leap:15.6
RUN zypper install -y curl
RUN zypper install -y curl && zypper clean
RUN zypper update -y curl
RUN zypper clean
RUN zypper cc
RUN echo zypper install curl
RUN cd /tmp && zypper install -y make
