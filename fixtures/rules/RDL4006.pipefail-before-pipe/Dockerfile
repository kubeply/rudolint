FROM alpine:3.20 AS missing
RUN wget -qO- https://example.com | tar xz
RUN test -f /tmp/nope || echo fallback
RUN test -f /tmp/nope||echo fallback

FROM alpine:3.20 AS configured
SHELL ["/bin/ash", "-o", "pipefail", "-c"]
RUN wget -qO- https://example.com | tar xz

FROM alpine:3.20 AS reset
RUN wget -qO- https://example.com | tar xz

FROM mcr.microsoft.com/powershell:latest AS powershell
SHELL ["pwsh", "-Command"]
RUN Invoke-WebRequest https://example.com | Out-Null
