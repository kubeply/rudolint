FROM alpine:3.20
ADD app.py /app/
ADD ./app /app
ADD --chown=app:app app.py /app/
ADD ["app.py", "/app/"]
ADD https://example.com/archive.tar.gz /tmp/archive.tar.gz
ADD rootfs.tar /
ADD rootfs.tar.gz /
ADD rootfs.tar.bz2 /
ADD rootfs.tar.xz /
ADD archive.tar app.py /app/
ADD https://example.com/archive.tar.gz app.py /app/
ADD "rootfs.tar.gz" /
ADD ["https://example.com/archive.tar.gz", "/tmp/archive.tar.gz"]
ADD ["rootfs.tar.gz", "/"]
ADD rootfs.gz /
ADD rootfs.bz2 /
ADD rootfs.xz /
