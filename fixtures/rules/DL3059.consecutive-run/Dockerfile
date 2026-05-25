FROM alpine:3.20
RUN echo one
RUN echo two

RUN --mount=type=cache,target=/var/cache/apk echo cached-one
RUN --mount=type=bind,target=/src echo cached-two

RUN echo alpha && echo beta && echo gamma
RUN echo after-large-run

RUN echo reset-before-copy
COPY file /file
RUN echo reset-after-copy
