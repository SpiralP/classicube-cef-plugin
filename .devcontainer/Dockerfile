FROM debian:bookworm

RUN export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y update \
    && apt-get -y install --no-install-recommends \
    # common
    curl ca-certificates git gcc g++ build-essential pkg-config \
    # openssl-sys <- reqwest
    libssl-dev \
    # bindgen <- classicube-sys
    clang-11 \
    # classicube-cef-plugin
    cmake \
    # cef ld linking
    libx11-dev \
    libnss3 libnspr4 libatk1.0-0 libatk-bridge2.0-0 libx11-xcb1 \
    libdrm2 libxkbcommon0 libdbus-1-3 libxcomposite1 libxdamage1 \
    libxext6 libxfixes3 libxrandr2 libgbm1 libpangocairo-1.0-0 libpango-1.0-0 \
    libasound2 libcups2 libatspi2.0-0 \
    && curl -sS 'https://sh.rustup.rs' | sh -s -- -y --default-toolchain none \
    && ~/.cargo/bin/rustup default nightly \
    && ~/.cargo/bin/rustup component add rls rust-analysis rust-src rustfmt clippy
