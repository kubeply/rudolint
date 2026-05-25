FROM debian:bookworm
RUN useradd -u 100000 app
RUN useradd --uid 100000 app
RUN useradd --uid=100000 app
RUN useradd -u100000 app
RUN useradd -u 99999 app
RUN useradd -u 000099 app
RUN useradd --uid uidvalue app
RUN useradd -l -u 100000 app
RUN useradd --no-log-init --uid=100000 app
RUN echo useradd -u 100000 app
ONBUILD RUN useradd -u 100000 child
