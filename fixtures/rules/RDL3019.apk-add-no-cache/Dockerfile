FROM alpine:3.20
RUN apk update
RUN apk add curl=8.14.1-r2
RUN apk add --no-cache curl=8.14.1-r2
RUN apk --no-cache add git=2.45.2-r0
RUN apk add --update make=4.4.1-r2
RUN echo apk add curl
RUN cd /tmp && apk add bash=5.2.26-r0
RUN apk --repository https://dl-cdn.alpinelinux.org/alpine/latest-stable/main add --no-cache curl=8.14.1-r2
RUN apk --repository add info curl
