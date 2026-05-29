FROM alpine:3.20
COPY app.py /app
COPY app.py config.yml /app
COPY app.py config.yml /app/
COPY app.py config.yml "/app/"
COPY app.py config.yml '${APP_DIR}/'
COPY ["app.py", "config.yml", "/app"]
COPY ["app.py", "config.yml", "/app/"]
COPY --chown=app:app app.py config.yml /app
COPY --from=builder /out/app /app
COPY --from=builder \
  /out/app /app
ADD app.py config.yml /app
COPY <<"FILE" /app/generated.sh
#!/usr/bin/env sh
echo generated
FILE
COPY <<FIRST <<SECOND /app/generated/
first body
FIRST
second body
SECOND
COPY --chmod=755 \
  <<'SCRIPT' /app/continued.sh
#!/usr/bin/env sh
echo continued
SCRIPT
