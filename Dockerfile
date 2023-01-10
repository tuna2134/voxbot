FROM rust:1 AS builder

WORKDIR /usr/src/build

RUN apt-get update && apt-get install -y \
    ffmpeg \
    cmake

COPY . .
RUN cargo build --release

FROM scratch

WORKDIR /usr/src/app

COPY --from=builder /usr/src/build/target/release/voxbot ./

CMD ["./voxbot"]
