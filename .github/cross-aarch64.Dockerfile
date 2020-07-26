# docker build -t cross-aarch64 -f .github/cross-aarch64.Dockerfile .
# cross build --target aarch64-unknown-linux-gnu --no-default-features

# ubuntu-like
FROM rustembedded/cross:aarch64-unknown-linux-gnu-0.2.1

# we're root
RUN dpkg --add-architecture arm64 \
    && apt-get -y update \
    && apt-get -y install curl git gcc g++ build-essential cmake clang-8 pkg-config \
    gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libc6-dev-i386 \
    libglib2.0-dev:arm64 libpango1.0-dev:arm64 libatk1.0-dev:arm64 libgtk-3-dev:arm64 libgdk-pixbuf2.0-dev:arm64 \
    libnss3:arm64 libasound2:arm64 libxss1:arm64 libnspr4:arm64 \
    && curl 'https://www.openssl.org/source/openssl-1.1.1g.tar.gz' |tar -xzf - \
    && cd openssl-1.1.1g \
    && export CROSS_COMPILE=aarch64-linux-gnu- \
    && ./Configure --prefix=/usr linux-aarch64 \
    && make -j4 \
    && make install_sw \
    # cleanup
    && apt-get -y autoremove && apt-get -y clean && rm -rf /var/lib/apt \
    && rm -rf /tmp && mkdir /tmp && chmod 777 /tmp \
    && rm -rf /usr/share/doc /usr/share/man /usr/share/locale

ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
