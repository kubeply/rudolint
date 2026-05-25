FROM opensuse/leap:15.6
RUN zypper install -y curl
RUN zypper update -y curl
RUN zypper dist-upgrade -y
RUN zypper dup -y
RUN echo zypper dist-upgrade -y
RUN cd /tmp && zypper dist-upgrade -y
RUN zypper --from dist-upgrade install -y curl
