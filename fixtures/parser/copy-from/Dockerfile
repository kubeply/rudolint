FROM alpine:3.20 AS build
FROM scratch
COPY --from=build /out/app /app
