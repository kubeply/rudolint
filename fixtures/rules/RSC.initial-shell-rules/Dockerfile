FROM alpine:3.20

# RSC2002: cat is only feeding a pipeline.
RUN cat app.log | grep error
# Good RSC2002: cat options make the pipeline meaningful.
RUN cat -n app.log | sed -n '1,10p'
# RSC2086: unquoted variable expansion.
RUN echo $HOME
# Good RSC2086: quoted variable expansion.
RUN echo "$HOME"
# RSC2046: unquoted command substitution.
RUN echo $(id -u)
# Good RSC2046: quoted command substitution.
RUN echo "$(id -u)"
# RSC2015: &&/|| chain.
RUN test -d /app && cd /app || mkdir /app
# Good RSC2015: no fallback branch.
RUN test -d /app && echo ok
# Good RSC2015: || starts after a command separator.
RUN test -d /app && echo ok; false || echo recovered
# RSC2155: export masks command substitution status.
RUN export PATH="$(pwd):$PATH"
# Good RSC2155: assignment and export are split.
RUN path="$(pwd)" && export path
# RSC2164: cd result is unchecked.
RUN cd /app; make
# Good RSC2164: cd is checked with &&.
RUN cd /app && make
# Good RSC2164: cd is guarded with ||.
RUN cd /app || exit 1
# RSC2181: checks the previous exit code indirectly.
RUN make; test $? -eq 0
# Good RSC2181: checks the command directly.
RUN make && echo ok
