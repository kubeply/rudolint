FROM alpine:3.20 AS unix-absolute
WORKDIR /app

FROM alpine:3.20 AS env-rooted
WORKDIR $APP_HOME

FROM mcr.microsoft.com/windows/nanoserver:ltsc2022 AS windows-absolute
WORKDIR C:\app

FROM alpine:3.20 AS plain-relative
WORKDIR app

FROM alpine:3.20 AS dot-relative
WORKDIR ./app

FROM alpine:3.20 AS parent-relative
WORKDIR ../app

FROM alpine:3.20 AS quoted-relative
WORKDIR "app"
