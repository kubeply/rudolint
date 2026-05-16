FROM alpine:3.20
COPY app.py /app
COPY app.py config.yml /app
COPY app.py config.yml /app/
COPY ["app.py", "config.yml", "/app"]
COPY ["app.py", "config.yml", "/app/"]
COPY --chown=app:app app.py config.yml /app
COPY --from=builder /out/app /app
ADD app.py config.yml /app
