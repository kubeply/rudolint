FROM alpine:3.20 AS base
COPY app.py app/
COPY app.py /app/
COPY app.py $APP_HOME/
COPY app.py C:\app
COPY <<"FILE" /generated.sh
#!/usr/bin/env sh
echo generated
FILE
COPY --chmod=755 \
  <<'SCRIPT' /continued.sh
#!/usr/bin/env sh
echo continued
SCRIPT
WORKDIR /app
COPY app.py .

FROM base AS inherited
COPY app.py src/

FROM alpine:3.20 AS fresh
COPY ["app.py", "src/"]
WORKDIR /src
COPY ["app.py", "."]

FROM alpine:3.20
WORKDIR /unnamed

FROM alpine:3.20
COPY app.py relative/

FROM 3 AS numeric-inherited
COPY app.py src/

FROM alpine:3.20 AS dev-mode-false
WORKDIR /workspace

FROM dev-mode-false AS dev-mode-true

FROM dev-mode-${MODE} AS dev-base

FROM dev-base AS dev
COPY app.py .

FROM alpine:3.20 AS no-workdir-mode-false

FROM alpine:3.20 AS no-workdir-mode-true

FROM no-workdir-mode-${MODE} AS no-workdir-base
COPY app.py .

FROM alpine:3.20 AS mixed-mode-false
WORKDIR /mixed

FROM alpine:3.20 AS mixed-mode-true

FROM mixed-mode-${MODE} AS mixed-base
COPY app.py .
