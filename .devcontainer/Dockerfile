FROM registry-1.docker.io/library/rust:1.93.0-trixie AS builder

ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /tmp

RUN git clone --depth 1 https://github.com/google/libphonenumber.git

RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    libprotobuf-dev \
    protobuf-compiler \
    libre2-dev \
    libicu-dev \
    libabsl-dev \
    libgtest-dev \
    default-jdk

WORKDIR /tmp/libphonenumber/cpp

RUN cmake \
    -DCMAKE_CXX_STANDARD=17 \
    -DUSE_BOOST=OFF \
    -DUSE_RE2=OFF \
    -DUSE_ICU_REGEXP=ON \
    -DCMAKE_INSTALL_PREFIX=/opt/libphonenumber \
    -DCMAKE_INSTALL_LIBDIR=lib \
    . \
    && make -j$(nproc) \
    && make install

FROM registry-1.docker.io/library/rust:1.93.0-trixie

ARG VARIANT=1.93.0-bookworm
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME
RUN rm /bin/sh && ln -s /bin/bash /bin/sh

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
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
    && apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /opt/libphonenumber/include /usr/local/include
COPY --from=builder /opt/libphonenumber/lib /usr/local/lib

RUN ldconfig
RUN chown -R vscode:vscode /usr/local/cargo
USER $USERNAME
RUN rustup component add clippy rustfmt