FROM rust:1.88-slim AS builder

WORKDIR /src/builder

ARG TARGETARCH
RUN if [ $TARGETARCH = "amd64" ]; then \
        echo "x86_64" > /tmp/arch; \
    elif [ $TARGETARCH = "arm64" ]; then \
        echo "aarch64" > /tmp/arch; \
    else \
        echo "Unsupported platform"; \
        exit 1; \
    fi

RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add $(cat /tmp/arch)-unknown-linux-musl

COPY . .
RUN --mount=type=cache,target=/src/builder/target/ cargo build --target=$(cat /tmp/arch)-unknown-linux-musl --release && \
  cp target/$(cat /tmp/arch)-unknown-linux-musl/release/expander /tmp/expander

FROM alpine AS get-certs

RUN apk update && apk upgrade

FROM scratch

WORKDIR /src/app

COPY --from=get-certs /etc/ssl/certs /etc/ssl/certs
COPY --from=builder /tmp/expander .

CMD ["./expander"]
