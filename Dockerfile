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
RUN --mount=type=cache,target=/src/builder/target/ if [ &TARGETARCH = "arm64" ]; then \
        export CC_aarch64_unknown_linux_musl=clang;
        export AR_aarch64_unknown_linux_musl=llvm-ar; \
        export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"; \
    fi \
    cargo build --target=$(cat /tmp/arch)-unknown-linux-musl --release && \
    cp target/$(cat /tmp/arch)-unknown-linux-musl/release/expander /tmp/expander

FROM scratch

WORKDIR /src/app

COPY --from=builder /tmp/expander .

CMD ["./expander"]
