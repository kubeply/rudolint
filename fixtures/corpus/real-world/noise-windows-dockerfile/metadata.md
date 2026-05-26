# Windows Dockerfile

This fixture protects common Windows Dockerfile syntax observed in large
multi-platform projects:

- Windows base images with explicit tags.
- Dockerfile escape directives using backticks.
- `SHELL ["cmd", "/S", "/C"]` followed by PowerShell commands.
- Windows paths and PowerShell environment variables such as `$Env:TEMP`.

It should stay quiet across profiles so POSIX shell rules do not regress into
Windows `RUN` instructions.

