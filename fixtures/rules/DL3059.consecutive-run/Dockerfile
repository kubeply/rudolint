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

FROM mcr.microsoft.com/windows/servercore:ltsc2022
SHELL ["powershell", "-command"]
RUN choco install make --version 4.3 -y
RUN choco install git --version 2.39.0 -y

FROM mcr.microsoft.com/windows/servercore:ltsc2022
SHELL ["cmd", "/S", "/C"]
RUN echo one
RUN echo two

FROM alpine:3.20
SHELL ["/bin/sh", "-c"]
RUN echo posix-one
RUN echo posix-two

FROM mcr.microsoft.com/windows/servercore:ltsc2022
SHELL powershell -command
RUN echo shell-form-one
RUN echo shell-form-two

FROM mcr.microsoft.com/windows/servercore:ltsc2022
SHELL "C:\Program Files\PowerShell\7\pwsh.exe" -command
RUN echo quoted-shell-form-one
RUN echo quoted-shell-form-two

FROM alpine:3.20
SHELL /bin/sh -c
RUN echo shell-form-posix-one
RUN echo shell-form-posix-two
