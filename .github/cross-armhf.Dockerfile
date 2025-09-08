# docker build --progress plain --pull -t cross-armhf -f .github/cross-armhf.Dockerfile . && cross build --target armv7-unknown-linux-gnueabihf

# ubuntu-like
# we need at least glibc 2.29
# 0.2.4 and 0.2.5 have glibc 2.23, so use main which has glibc 2.31
# https://github.com/cross-rs/cross/pkgs/container/aarch64-unknown-linux-gnu
FROM ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:main@sha256:200e97ef1a9c4bee80a772a2d7cb10865d384b957f144ec3ec19d2648a36921b

# we're root
RUN export DEBIAN_FRONTEND=noninteractive \
    && dpkg --add-architecture armhf \
    && apt-get -y update \
    && apt-get -y install wget curl git gcc g++ build-essential cmake clang pkg-config \
    gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf libc6-dev-i386 \
    libssl-dev:armhf libglib2.0-dev:armhf libpango1.0-dev:armhf libatk1.0-dev:armhf libgtk-3-dev:armhf libgdk-pixbuf2.0-dev:armhf \
    libnss3:armhf libasound2:armhf libxss1:armhf libnspr4:armhf \
    && apt-get -y autoremove && apt-get -y clean && rm -rf /var/lib/apt \
    && rm -rf /tmp && mkdir /tmp && chmod 777 /tmp \
    && rm -rf /usr/share/doc /usr/share/man /usr/share/locale

ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig
ENV CPLUS_INCLUDE_PATH=/usr/arm-linux-gnueabihf/include/c++/5/arm-linux-gnueabihf

# bits/c++config.h not found
RUN ln -vs /usr/arm-linux-gnueabihf/include/c++/9/arm-linux-gnueabihf/bits/* /usr/include/c++/9/bits/
