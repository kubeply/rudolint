FROM alpine:3.20 AS missing
RUN wget -qO- https://example.com | tar xz
RUN test -f /tmp/nope || echo fallback
RUN test -f /tmp/nope||echo fallback

FROM alpine:3.20 AS configured
SHELL ["/bin/ash", "-o", "pipefail", "-c"]
RUN wget -qO- https://example.com | tar xz

FROM alpine:3.20 AS reset
RUN wget -qO- https://example.com | tar xz

FROM alpine:3.20 AS shell-form
SHELL /bin/bash -o pipefail -c
RUN wget -qO- https://example.com | tar xz

FROM mcr.microsoft.com/powershell:latest AS powershell
SHELL ["pwsh", "-Command"]
RUN Invoke-WebRequest https://example.com | Out-Null

FROM mcr.microsoft.com/powershell:latest AS shell-form-powershell
SHELL C:\Windows\System32\pwsh.exe -Command
RUN Invoke-WebRequest https://example.com | Out-Null

FROM alpine:3.20 AS quoted
RUN echo 'a|b'
RUN echo "a|b"
RUN echo a\|b
