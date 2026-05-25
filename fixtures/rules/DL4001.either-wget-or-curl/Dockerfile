FROM alpine:3.20 AS mixed
RUN curl -fsSLO https://example.com/a
RUN wget https://example.com/b
RUN curl -fsSLO https://example.com/c

FROM alpine:3.20 AS reset
RUN wget https://example.com/a
RUN wget https://example.com/b

FROM alpine:3.20 AS same-run
RUN curl -fsSLO https://example.com/a && wget https://example.com/b
