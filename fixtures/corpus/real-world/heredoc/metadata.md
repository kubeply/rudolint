# Heredoc

This fixture represents shell setup logic embedded with Dockerfile heredocs.

It exists to protect heredoc parsing, line spans, and BuildKit feature detection
for realistic multiline `RUN` instructions.
