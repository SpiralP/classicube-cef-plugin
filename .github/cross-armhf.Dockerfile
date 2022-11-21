# docker build --pull -t cross-armhf -f .github/cross-armhf.Dockerfile .
# cross build --target armv7-unknown-linux-gnueabihf

# ubuntu-like
FROM ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:0.2.4

# we're root
RUN export DEBIAN_FRONTEND=noninteractive \
    && dpkg --add-architecture armhf \
    && apt-get -y update \
    && apt-get -y install \
    wget curl git gcc g++ build-essential cmake clang-8 pkg-config \
    gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf libc6-dev-i386 \
    libglib2.0-dev:armhf libpango1.0-dev:armhf libatk1.0-dev:armhf libgtk-3-dev:armhf libgdk-pixbuf2.0-dev:armhf \
    libnss3:armhf libasound2:armhf libxss1:armhf libnspr4:armhf \
    && wget -O /tmp/cmake-install.sh 'https://github.com/Kitware/CMake/releases/download/v3.25.0/cmake-3.25.0-linux-x86_64.sh' \
    && sh /tmp/cmake-install.sh --skip-license --prefix=/usr/local \
    && rm /tmp/cmake-install.sh \
    && curl 'https://www.openssl.org/source/openssl-1.1.1s.tar.gz' |tar -xzf - \
    && cd openssl-1.1.1s \
    && export CROSS_COMPILE=arm-linux-gnueabihf- \
    && ./Configure --prefix=/usr linux-generic32 \
    && make -j4 \
    && make install_sw \
    # cleanup
    && cd .. && rm -rf openssl-1.1.1s \
    && apt-get -y autoremove && apt-get -y clean && rm -rf /var/lib/apt \
    && rm -rf /tmp && mkdir /tmp && chmod 777 /tmp \
    && rm -rf /usr/share/doc /usr/share/man /usr/share/locale

ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig
ENV CPLUS_INCLUDE_PATH=/usr/arm-linux-gnueabihf/include/c++/5/arm-linux-gnueabihf
