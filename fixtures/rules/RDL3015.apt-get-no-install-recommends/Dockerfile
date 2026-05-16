FROM ubuntu:24.04
RUN apt-get update
RUN apt-get install -y curl=8.5.0-2ubuntu10.6
RUN apt-get --no-install-recommends install -y curl=8.5.0-2ubuntu10.6
RUN apt-get install -y --no-install-recommends curl=8.5.0-2ubuntu10.6
RUN echo apt-get install curl
RUN apt-get -o Dir::Cache=install update
RUN cd /tmp && apt-get install -y make=4.3-4.1build2
