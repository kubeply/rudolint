FROM alpine:3.20 AS build
RUN mkdir -p /out && touch /out/app

FROM alpine:3.20 AS runtime
COPY --from=build /out/app /app
COPY --from=0 /out/app /app-index
COPY --from=missing /out/app /app-missing
COPY --from=9 /out/app /app-bad-index
COPY --from=alpine:3.20 /etc/os-release /os-release

FROM alpine:3.20 AS later
COPY --from=runtime /app /app-copy
COPY --from=future /app /future

FROM alpine:3.20 AS future
COPY app.py /app/
