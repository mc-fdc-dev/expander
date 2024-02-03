FROM rust:slim AS builder

WORKDIR /src/builder

RUN apt-get update && apt-get install -y musl-tools

ARG TARGETARCH
RUN if [ $TARGETARCH = "amd64" ]; then \
        echo "x86_64" > /tmp/arch; \
    elif [ $TARGETARCH = "arm64" ]; then \
        echo "aarch64" > /tmp/arch; \
        apt-get install -y clang llvm; \
    else \
        echo "Unsupported platform"; \
        exit 1; \
    fi

RUN rustup target add $(cat /tmp/arch)-unknown-linux-musl

COPY . .
RUN --mount=type=cache,target=/src/builder/target/ bash build.sh

FROM alpine

WORKDIR /src/app

COPY --from=builder /tmp/expander .

CMD ["./expander"]
