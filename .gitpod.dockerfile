FROM gitpod/workspace-full

RUN sudo apt-get update; sudo apt-get install -y pkg-config libx11-dev libasound2-dev libudev-dev
RUN rustup default beta
RUN cargo install cargo-make
RUN cargo install wasm-pack
RUN cargo install http-server
