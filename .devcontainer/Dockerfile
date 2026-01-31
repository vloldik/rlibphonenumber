FROM registry-1.docker.io/library/rust:1.93.0-bookworm

ARG VARIANT=1.93.0-bookworm
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
       build-essential \
       cmake \
       pkg-config \
       libssl-dev \
       lldb
RUN chown -R vscode:vscode /usr/local/cargo
USER $USERNAME

RUN rustup component add clippy rustfmt