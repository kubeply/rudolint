FROM rockylinux:9
RUN yum install -y curl
RUN yum install -y curl && yum clean all
RUN yum clean all
RUN yum update -y curl
RUN echo yum install curl
RUN cd /tmp && yum install -y make
RUN yum --installroot /mnt install -y tar
RUN yum --installroot /mnt install -y gzip && yum --installroot /mnt clean all
RUN yum --config install update curl
RUN yum clean all && yum install -y unzip
RUN yum install -y findutils && yum clean all && yum install -y diffutils
