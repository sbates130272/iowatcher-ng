FROM rust:1.65.0
RUN apt-get update && \
    apt-get install -y \
    g++ \
    cmake \
    llvm-dev \
    clang \
    libclang-dev
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM ubuntu:22.04
RUN apt-get update && apt-get -y install netcat
WORKDIR /usr/bin
COPY --from=0 /usr/src/myapp/target/release/iowatcherng ./iowatcherng-exporter
EXPOSE 9975
CMD [ "iowatcherng-exporter" ]
