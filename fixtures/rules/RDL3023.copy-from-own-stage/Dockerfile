FROM alpine:3.20 AS build
RUN mkdir -p /out && touch /out/app
COPY --from=build /out/app /self
COPY --from=0 /out/app /self-index

FROM alpine:3.20 AS runtime
COPY --from=build /out/app /app
COPY --from=runtime /app /self
COPY --from=RUNTIME /app /self-uppercase
COPY --from=0 /out/app /numeric
COPY --from=1 /app /self-index
COPY --from=alpine:3.20 /etc/os-release /os-release

FROM alpine:3.20
COPY --from=runtime /app /app-copy
