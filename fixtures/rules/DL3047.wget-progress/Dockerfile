FROM alpine:3.20
RUN wget https://example.com/archive.tar.gz
RUN wget --progress=dot:giga https://example.com/archive.tar.gz
RUN wget -q https://example.com/archive.tar.gz
RUN wget -qT 30 https://example.com/archive.tar.gz
RUN wget -Oqfile.txt https://example.com/archive.tar.gz
RUN wget --quiet https://example.com/archive.tar.gz
RUN wget -nv https://example.com/archive.tar.gz
RUN wget --no-verbose https://example.com/archive.tar.gz
RUN wget -o wget.log https://example.com/archive.tar.gz
RUN wget --output-file wget.log https://example.com/archive.tar.gz
RUN wget --output-file=wget.log https://example.com/archive.tar.gz
RUN wget -a wget.log https://example.com/archive.tar.gz
RUN wget --append-output wget.log https://example.com/archive.tar.gz
RUN wget --append-output=wget.log https://example.com/archive.tar.gz
RUN echo wget https://example.com/archive.tar.gz
ONBUILD RUN wget https://example.com/onbuild.tar.gz
