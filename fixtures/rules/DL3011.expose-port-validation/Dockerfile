FROM alpine:3.20
# Valid: lower boundary in range.
EXPOSE 0
# Valid: in-range ports with optional protocol.
EXPOSE 80 443/tcp 53/udp
# Valid: upper boundary in range.
EXPOSE 65535
# Valid: Docker accepts port ranges in EXPOSE.
EXPOSE 80-90 9500-9999/tcp ${PORT_START}-${PORT_END}
# Valid: Dockerfile environment substitution is supported in EXPOSE.
EXPOSE $PORT ${PORT_SSL}/tcp
# Valid: multiline EXPOSE should ignore continuation markers.
EXPOSE ${PORT_HTTP} \
       ${PORT_HTTPS}
# Invalid: port exceeds 65535.
EXPOSE 65536
# Invalid: port exceeds 65535 with protocol.
EXPOSE 70000/tcp
# Invalid: negative ports are not numeric.
EXPOSE -1
# Invalid: port is non-numeric.
EXPOSE port
# Invalid: range start exceeds range end.
EXPOSE 90-80
# Invalid: range end exceeds 65535.
EXPOSE 65000-70000
