FROM alpine:3.20 AS missing
RUN echo missing

FROM alpine:3.20 AS healthy
HEALTHCHECK CMD wget -qO- http://localhost/health || exit 1

FROM healthy AS inherited
RUN echo inherited

FROM 1 AS numeric_inherited
RUN echo numeric_inherited

FROM alpine:3.20
HEALTHCHECK CMD wget -qO- http://localhost/health || exit 1

FROM alpine:3.20 AS same_image_missing
RUN echo same_image_missing

FROM missing AS repaired
HEALTHCHECK NONE

FROM missing AS still_missing
RUN echo still_missing

FROM debian:12 AS missing_final
RUN echo missing_final
