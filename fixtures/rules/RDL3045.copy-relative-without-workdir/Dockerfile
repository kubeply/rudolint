FROM alpine:3.20 AS base
COPY app.py app/
COPY app.py /app/
COPY app.py $APP_HOME/
COPY app.py C:/app/
WORKDIR /app
COPY app.py .

FROM base AS inherited
COPY app.py src/

FROM alpine:3.20 AS fresh
COPY ["app.py", "src/"]
WORKDIR /src
COPY ["app.py", "."]
