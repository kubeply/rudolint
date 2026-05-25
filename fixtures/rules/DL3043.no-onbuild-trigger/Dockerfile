FROM alpine:3.20
ONBUILD RUN echo ok
ONBUILD COPY . /src
ONBUILD ONBUILD RUN echo nested
ONBUILD FROM scratch
ONBUILD MAINTAINER ops@example.com
ONBUILD from busybox
