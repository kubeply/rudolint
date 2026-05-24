FROM alpine:3.20
RUN <<EOF
echo hello
EOF
RUN <<'SCRIPT'
echo quoted
SCRIPT
COPY <<"FILE" /usr/local/bin/generated
#!/usr/bin/env sh
echo generated
FILE
COPY <<FIRST <<SECOND /etc/generated/
first body
FIRST
second body
SECOND
COPY --chmod=755 \
  <<'SCRIPT' /usr/local/bin/continued
#!/usr/bin/env sh
echo continued
SCRIPT
COPY <<FIRST \
  <<SECOND /etc/continued-generated/
first continued body
FIRST
second continued body
SECOND
