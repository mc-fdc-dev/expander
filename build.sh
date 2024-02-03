#!/bin/bash
if [ $(cat /tmp/arch) = "aarch64" ]; then
    export CC_aarch64_unknown_linux_musl=clang
    export AR_aarch64_unknown_linux_musl=llvm-ar
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-Clink-self-contained=yes -Clinker=rust-lld"
else
    echo "amd64 mode"
fi
cargo build --target=$(cat /tmp/arch)-unknown-linux-musl --release
cp target/$(cat /tmp/arch)-unknown-linux-musl/release/expander /tmp/expander
