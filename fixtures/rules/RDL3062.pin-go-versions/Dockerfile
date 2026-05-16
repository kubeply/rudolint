FROM golang:1.23-alpine
RUN go install example.com/tool/cmd/tool
RUN go install example.com/tool/cmd/tool@v1.2.3
RUN go install example.com/tool/cmd/tool@latest
RUN go get example.com/library
RUN go get example.com/library@none
RUN go run example.com/script
RUN go run example.com/script@v0.4.0 --flag value
RUN go run main.go
RUN go run -mod mod main.go
RUN go run -mod mod example.com/script
RUN go install -mod mod example.com/tool/cmd/tool@v1.2.3
RUN go install -mod mod example.com/tool/cmd/tool
RUN go run ./cmd/local
RUN go install ./cmd/local
RUN go install \
    example.com/tool/cmd/tool@v1.2.3
RUN go install -mod \
    mod example.com/tool/cmd/tool@v1.2.3
RUN go install example.com/tool/cmd/tool@
RUN go -C /src install example.com/tool/cmd/tool@v1.2.3
RUN go -C /src install example.com/tool/cmd/tool
RUN go install -buildmode pie example.com/tool/cmd/tool@v1.2.3
RUN go install -compiler gc example.com/tool/cmd/tool@v1.2.3
RUN go install -gccgoflags -O2 example.com/tool/cmd/tool@v1.2.3
RUN go install -installsuffix static example.com/tool/cmd/tool@v1.2.3
