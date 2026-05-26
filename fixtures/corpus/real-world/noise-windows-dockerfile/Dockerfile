# escape=`
FROM mcr.microsoft.com/windows/servercore:ltsc2022

SHELL ["cmd", "/S", "/C"]
WORKDIR C:\hadoop

COPY pkg-resolver .\pkg-resolver

RUN powershell -NoProfile -ExecutionPolicy Bypass `
    $ProgressPreference = 'SilentlyContinue'; `
    New-Item -ItemType Directory -Force -Path $Env:TEMP\hadoop | Out-Null; `
    Invoke-WebRequest -UseBasicParsing -Uri https://example.invalid/hadoop.zip -OutFile $Env:TEMP\hadoop\archive.zip

RUN if not exist C:\hadoop\bin mkdir C:\hadoop\bin

