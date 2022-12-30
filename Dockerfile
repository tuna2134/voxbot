FROM rust:1

WORKDIR /usr/src/app

RUN apt-get update && apt-get install -y \
    ffmpeg \
    cmake

COPY . .
RUN cargo build --release
COPY ./target/release/ /usr/src/app/target/release/

CMD ["./target/release/voxbot"]