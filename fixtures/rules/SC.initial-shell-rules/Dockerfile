FROM alpine:3.20

# SC2002: cat is only feeding a pipeline.
RUN cat app.log | grep error
# Good SC2002: cat options make the pipeline meaningful.
RUN cat -n app.log | sed -n '1,10p'
# SC2086: unquoted variable expansion.
RUN echo $HOME
# Good SC2086: quoted variable expansion.
RUN echo "$HOME"
# SC2046: unquoted command substitution.
RUN echo $(id -u)
# Good SC2046: quoted command substitution.
RUN echo "$(id -u)"
# Good SC2046: command substitution in simple assignments does not split.
RUN SYSTEM=$(uname -s | tr '[:upper:]' '[:lower:]')
RUN PLATFORM=$([ "$(uname -m)" = "aarch64" ] && echo "arm64" || echo "amd64")
# SC2015: &&/|| chain.
RUN test -d /app && cd /app || mkdir /app
# Good SC2015: no fallback branch.
RUN test -d /app && echo ok
# Good SC2015: || starts after a command separator.
RUN test -d /app && echo ok; false || echo recovered
# SC2155: export masks command substitution status.
RUN export PATH="$(pwd):$PATH"
# Good SC2155: assignment and export are split.
RUN path="$(pwd)" && export path
# SC2164: cd result is unchecked.
RUN cd /app; make
# Good SC2164: cd is checked with &&.
RUN cd /app && make
# Good SC2164: cd is guarded with ||.
RUN cd /app || exit 1
# SC2181: checks the previous exit code indirectly.
RUN make; test $? -eq 0
# Good SC2181: checks the command directly.
RUN make && echo ok
