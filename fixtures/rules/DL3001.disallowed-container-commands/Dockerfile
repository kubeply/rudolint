FROM alpine:3.20
RUN ssh localhost
RUN apk add --no-cache vim
RUN cd /tmp && vim file
RUN FOO=bar /usr/bin/service nginx start
RUN ps aux | grep nginx
RUN printf '%s' kill
RUN mount -t proc proc /proc; ifconfig
