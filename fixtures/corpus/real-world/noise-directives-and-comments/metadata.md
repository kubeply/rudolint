# Directives And Comments Noise

This fixture represents Dockerfiles with parser directives, check directives,
and regular comments around otherwise simple instructions.

It exists to catch false positives caused by comment handling, directives, and
JSON-form `RUN` or `CMD` instructions.
