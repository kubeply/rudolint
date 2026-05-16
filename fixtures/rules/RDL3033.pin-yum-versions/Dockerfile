FROM rockylinux:9
RUN yum install -y curl
RUN yum install -y curl-8.0.1
RUN yum install -y ca-certificates
RUN yum install -y python3-devel-3.9.18
RUN yum install -y curl-8.0.1 git
RUN yum update -y curl
RUN echo yum install curl
RUN cd /tmp && yum install -y make
RUN yum --installroot /mnt install -y curl-8.0.1
RUN yum --installroot /mnt install -y curl
RUN yum --config install update curl
