#!/bin/bash
if [ &TARGETARCH = "arm64" ]; then
    CC_aarch64_unknown_linux_musl=clang
    AR_aarch64_unknown_linux_musl=llvm-ar
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"
else
    echo "amd64 mode"
fi
cargo build --target=$(cat /tmp/arch)-unknown-linux-musl --release
cp target/$(cat /tmp/arch)-unknown-linux-musl/release/expander /tmp/expander
