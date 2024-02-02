FROM rust AS builder

WORKDIR /src/builder

RUN apt-get update && apt-get install -y musl-tools
RUN rustup target add aarch64-unknown-linux-musl

COPY . .
RUN --mount=type=cache,target=/src/builder/target/aarch64-unknown-linux-musl/release/ cargo build --target=aarch64-unknown-linux-musl --release

FROM alpine

WORKDIR /src/app

RUN apk add ca-certificates
COPY --from=builder /src/builder/target/aarch64-unknown-linux-musl/release/expander .

CMD ["./expander"]
