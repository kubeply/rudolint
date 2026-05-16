FROM alpine:3.20
COPY app.py /app/
COPY rootfs.tar /
COPY rootfs.tar.gz /
COPY --chown=root:root rootfs.tar.xz /
ADD rootfs.tar.gz /
