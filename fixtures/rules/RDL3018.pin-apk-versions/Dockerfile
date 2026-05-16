FROM alpine:3.20
RUN apk update
RUN apk add curl
RUN apk add curl=8.14.1-r2
RUN apk add --no-cache curl=8.14.1-r2 git
RUN echo apk add curl
RUN cd /tmp && apk add make
RUN apk --repository https://dl-cdn.alpinelinux.org/alpine/latest-stable/main add curl=8.14.1-r2
RUN apk add --virtual .build-deps gcc=14.2.0-r4
RUN apk --repository add info curl
