FROM alpine:3.20
CMD echo hello
ENTRYPOINT ./entrypoint.sh --flag
CMD ["echo", "hello"]
ENTRYPOINT ["/entrypoint.sh"]
HEALTHCHECK CMD wget -qO- http://localhost/health || exit 1
