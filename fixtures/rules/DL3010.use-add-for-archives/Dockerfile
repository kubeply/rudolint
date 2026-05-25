FROM alpine:3.20
COPY app.py /app/
COPY rootfs.tar /
COPY rootfs.tar.gz /
COPY --chown=root:root rootfs.tar.xz /
COPY --from=build rootfs.tar.gz /
COPY --from build rootfs.tar /
ADD rootfs.tar.gz /
