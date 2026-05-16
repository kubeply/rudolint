FROM alpine:3.20
WORKDIR /app
RUN make
RUN cd /app && make
RUN mkdir -p /opt/app && cd /opt/app && make
RUN printf '%s' cd
