FROM rust:1 AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev

WORKDIR /usr/src/build

RUN apt-get update && apt-get install -y \
    ffmpeg \
    cmake

COPY . .
RUN cargo build --release

FROM scratch

WORKDIR /usr/src/app

COPY --from=builder /usr/src/build/target/x86_64-unknown-linux-musl/release/voxbot ./

CMD ["./voxbot"]
