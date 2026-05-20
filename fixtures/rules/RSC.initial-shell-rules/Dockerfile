FROM alpine:3.20
RUN echo $HOME
RUN echo "$HOME"
RUN echo $(id -u)
RUN echo "$(id -u)"
RUN test -d /app && cd /app || mkdir /app
RUN test -d /app && echo ok
RUN export PATH="$(pwd):$PATH"
RUN path="$(pwd)" && export path
RUN cd /app; make
RUN cd /app && make
RUN cd /app || exit 1
