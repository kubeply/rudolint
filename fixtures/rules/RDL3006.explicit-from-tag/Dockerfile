FROM alpine

FROM library/alpine

FROM ghcr.io/kubeply/base

FROM localhost:5000/alpine

FROM --platform=$BUILDPLATFORM debian

FROM alpine:latest

FROM alpine:3.20 AS tagged

FROM ubuntu@sha256:2c047404e52a3cfceb09e5c46f73076d8a3cf9931d9b145438cdd093b87aa970

FROM scratch AS empty

FROM tagged AS copied_from_stage

FROM empty AS copied_from_scratch_stage

FROM alpine:3.20 AS MixedTag

FROM mixedtag AS copied_from_mixed_case_stage
