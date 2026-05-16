FROM alpine:3.20 AS base
COPY app.py app/
COPY app.py /app/
COPY app.py $APP_HOME/
COPY app.py C:\app
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
