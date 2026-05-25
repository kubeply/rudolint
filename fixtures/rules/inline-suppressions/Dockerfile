# rudolint ignore=DL3007
FROM alpine:latest

# rudolint ignore=DL3000
WORKDIR relative

# rudolint ignore=RDK1001
ARG AWS_SECRET_ACCESS_KEY=example

# rudolint ignore=all
RUN apt-get update && apt-get install -y curl

ADD app.py /app/
