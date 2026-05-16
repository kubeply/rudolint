FROM opensuse/leap:15.6
RUN zypper install curl
RUN zypper install -y curl
RUN zypper -n install curl
RUN zypper --non-interactive install curl
RUN zypper update curl
RUN echo zypper install curl
RUN cd /tmp && zypper install make
RUN zypper --root /mnt install tar
RUN zypper --root /mnt install -y gzip
RUN zypper --root install update curl
RUN zypper -R /mnt install unzip
RUN zypper -c /tmp/zypp.conf install -y patch
RUN zypper in bzip2
RUN zypper in -y xz
