# docker build --progress plain --pull -t cross-aarch64 -f .github/cross-aarch64.Dockerfile . && cross build --target aarch64-unknown-linux-gnu

# ubuntu-like
# we need at least glibc 2.29
# 0.2.4 and 0.2.5 have glibc 2.23, so use main which has glibc 2.31
# https://github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu
FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:main@sha256:3bf094d22fc4f73c9bdce45ddd7a8bbae349efdbd51b4d4b5ee1bedd8454466b

# we're root
RUN export DEBIAN_FRONTEND=noninteractive \
    && dpkg --add-architecture arm64 \
    && apt-get -y update \
    && apt-get -y install wget curl git gcc g++ build-essential cmake clang pkg-config \
    gcc-aarch64-linux-gnu g++-aarch64-linux-gnu \
    gcc-10-aarch64-linux-gnu g++-10-aarch64-linux-gnu \
    libc6-dev-i386 \
    libssl-dev:arm64 libglib2.0-dev:arm64 libpango1.0-dev:arm64 libatk1.0-dev:arm64 libgtk-3-dev:arm64 libgdk-pixbuf2.0-dev:arm64 \
    libnss3:arm64 libasound2:arm64 libxss1:arm64 libnspr4:arm64 \
    && apt-get -y autoremove && apt-get -y clean && rm -rf /var/lib/apt \
    && rm -rf /tmp && mkdir /tmp && chmod 777 /tmp \
    && rm -rf /usr/share/doc /usr/share/man /usr/share/locale

# CEF 145 requires C++20 (<concepts> header), which needs GCC 10+
RUN update-alternatives --install /usr/bin/aarch64-linux-gnu-gcc aarch64-linux-gnu-gcc /usr/bin/aarch64-linux-gnu-gcc-10 100 \
    && update-alternatives --install /usr/bin/aarch64-linux-gnu-g++ aarch64-linux-gnu-g++ /usr/bin/aarch64-linux-gnu-g++-10 100

ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
ENV CPLUS_INCLUDE_PATH=/usr/aarch64-linux-gnu/include/c++/10:/usr/aarch64-linux-gnu/include/c++/10/aarch64-linux-gnu

# bits/c++config.h not found
RUN mkdir -p /usr/include/c++/10/bits \
    && ln -vs /usr/aarch64-linux-gnu/include/c++/10/aarch64-linux-gnu/bits/* /usr/include/c++/10/bits/
