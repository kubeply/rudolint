FROM alpine:3.20 AS build
FROM scratch
COPY --from=build /out/app /app
COPY --chown=1000:1000 --chmod=0755 bin/rudolint /usr/local/bin/rudolint
COPY ["dir with spaces/file.txt", "/opt/dir with spaces/file.txt"]
ADD https://example.com/archive.tar.gz /tmp/archive.tar.gz
