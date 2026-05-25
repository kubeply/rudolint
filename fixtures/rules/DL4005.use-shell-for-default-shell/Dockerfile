FROM alpine:3.20
RUN ln -sf /bin/bash /bin/sh
RUN ln -s /bin/sh /usr/local/bin/sh
RUN ln /bin/bash /bin/sh
RUN ln -sf /usr/bin/tool /usr/local/bin/tool
SHELL ["/bin/ash", "-c"]
RUN echo ok
