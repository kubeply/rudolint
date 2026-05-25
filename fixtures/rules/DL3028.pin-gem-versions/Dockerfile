FROM ruby:3.3
RUN gem install bundler
RUN gem install bundler:2.5.11
RUN gem install --no-document rake
RUN gem install --no-document rake:13.2.1
RUN gem install rails -v 7.1.3
RUN gem update bundler
RUN echo gem install bundler
RUN cd /tmp && gem install json
RUN gem install --platform ruby rake:13.2.1
RUN gem install --platform ruby rake
RUN gem --config-file /tmp/gemrc install rake:13.2.1
RUN gem --config-file install list bundler
RUN gem install bundler -v
RUN gem install bundler --version=
