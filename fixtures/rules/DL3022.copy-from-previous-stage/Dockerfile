FROM alpine:3.20 AS build
RUN mkdir -p /out && touch /out/app

FROM alpine:3.20 AS runtime
COPY --from=build /out/app /app
COPY --from=0 /out/app /app-index
COPY --from=missing /out/app /app-missing
COPY --from=9 /out/app /app-bad-index
COPY --from=alpine:3.20 /etc/os-release /os-release
COPY --from=registry:5000/example/app:1.0 /etc/os-release /tagged-registry-image
COPY --from=registry:5000/example/app /etc/os-release /untagged-registry-image

FROM alpine:3.20 AS later
COPY --from=runtime /app /app-copy
COPY --from=future /app /future
COPY --from=1 /app /self-index

FROM alpine:3.20 AS MixedCase
COPY --from=MixedCase /etc/os-release /mixed-case-self

FROM alpine:3.20 AS future
COPY --from=MixedCase /etc/os-release /mixed-case
COPY --from=MIXEDCASE /etc/os-release /mixed-case-upper
COPY app.py /app/
