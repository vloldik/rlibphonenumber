ARG RUST_VERSION=1.93.0-trixie
FROM registry-1.docker.io/library/rust:${RUST_VERSION}

ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID
ARG DAGGER_VERSION=0.20.5

ENV DEBIAN_FRONTEND=noninteractive

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME
RUN rm /bin/sh && ln -s /bin/bash /bin/sh

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    lldb \
    protobuf-compiler \
    libprotobuf-dev \
    libre2-dev \
    libicu-dev \
    libabsl-dev \
    sudo \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

RUN curl -L https://dl.dagger.io/dagger/install.sh \
    | VERSION=${DAGGER_VERSION} BIN_DIR=/usr/local/bin sh

RUN echo "$USERNAME ALL=(ALL) NOPASSWD: /usr/local/share/docker-init.sh" > /etc/sudoers.d/vscode

RUN chown -R $USERNAME:$USERNAME /usr/local/cargo

USER $USERNAME
RUN rustup component add clippy rustfmt \
    && rustup toolchain install nightly \
    && cargo install cargo-fuzz --locked
