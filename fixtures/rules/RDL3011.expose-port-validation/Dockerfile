FROM alpine:3.20
# Valid: lower boundary in range.
EXPOSE 0
# Valid: in-range ports with optional protocol.
EXPOSE 80 443/tcp 53/udp
# Valid: upper boundary in range.
EXPOSE 65535
# Invalid: port exceeds 65535.
EXPOSE 65536
# Invalid: port exceeds 65535 with protocol.
EXPOSE 70000/tcp
# Invalid: negative ports are not numeric.
EXPOSE -1
# Invalid: port is non-numeric.
EXPOSE port
# Invalid: port range syntax is not supported.
EXPOSE 80-90
