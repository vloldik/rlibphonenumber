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
    libicu-dev \
    libabsl-dev \
    libgtest-dev \
    default-jdk \
    maven

WORKDIR /tmp/libphonenumber/cpp

RUN curl -sSL https://github.com/google/re2/archive/refs/tags/2022-12-01.tar.gz | tar -xz \
    && cd re2-2022-12-01 \
    && mkdir build && cd build \
    && cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_POSITION_INDEPENDENT_CODE=ON .. \
    && make -j$(nproc) \
    && make install \
    && cd ../.. && rm -rf re2-2022-12-01

RUN sed -i 's/\bStringPiece\b/re2::StringPiece/g' src/phonenumbers/regexp_adapter_re2.cc

RUN cmake \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_CXX_FLAGS="-O3 -DNDEBUG" \
    -DCMAKE_CXX_STANDARD=17 \
    -DUSE_BOOST=OFF \
    -DUSE_RE2=ON \
    -DUSE_ICU_REGEXP=OFF \
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
        maven \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /opt/libphonenumber/include /usr/local/include
COPY --from=builder /opt/libphonenumber/lib /usr/local/lib

RUN ldconfig
RUN chown -R vscode:vscode /usr/local/cargo
USER $USERNAME
RUN rustup component add clippy rustfmt