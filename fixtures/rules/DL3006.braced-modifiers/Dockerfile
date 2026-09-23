ARG BASE=alpine
ARG EMPTY=
ARG MISSING
ARG FALLBACK=busybox

FROM ${BASE:-busybox:1.36} AS set_base
FROM ${EMPTY:-busybox} AS empty_base
FROM ${MISSING:-busybox:1.36} AS tagged_fallback
FROM ${BASE-busybox:1.36} AS set_hyphen
FROM ${BASE:+busybox} AS alternate
FROM ${BASE+busybox} AS plus
FROM ${MISSING:-${FALLBACK}} AS nested_fallback
