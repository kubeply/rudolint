FROM ubuntu:24.04
RUN apt-get update
RUN apt-get install -y curl
RUN apt-get install -y curl=8.5.0-2ubuntu10.6
RUN apt-get install --no-install-recommends curl=8.5.0-2ubuntu10.6 ca-certificates
RUN apt-get install -t noble curl=8.5.0-2ubuntu10.6
RUN apt-get install --target-release noble curl=8.5.0-2ubuntu10.6
RUN apt-get install -c /tmp/apt.conf curl=8.5.0-2ubuntu10.6
RUN echo apt-get install curl
RUN cd /tmp && apt-get install -y git=1:2.43.0-1ubuntu7.2 make
