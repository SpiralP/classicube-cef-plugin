# docker build -t cross-armhf -f .github/cross-armhf.Dockerfile .
# cross build --target arm-unknown-linux-gnueabihf --no-default-features

# ubuntu-like
FROM rustembedded/cross:arm-unknown-linux-gnueabihf-0.2.1

# we're root
RUN dpkg --add-architecture armhf \
    && apt-get -y update \
    && apt-get -y install curl git gcc g++ build-essential cmake clang-10 pkg-config \
    libc6-dev-i386 \
    libglib2.0-dev:armhf libpango1.0-dev:armhf libatk1.0-dev:armhf libgtk-3-dev:armhf libgdk-pixbuf2.0-dev:armhf \
    libnss3:armhf libasound2:armhf libxss1:armhf libnspr4:armhf \
    && curl 'https://www.openssl.org/source/openssl-1.1.1g.tar.gz' |tar -xzf - \
    && cd openssl-1.1.1g \
    && export CROSS_COMPILE=arm-linux-gnueabihf- \
    && ./Configure --prefix=/usr/arm-linux-gnueabihf linux-generic32 \
    && make -j4 \
    && make install_sw \
    # cleanup
    && cd .. && rm -rf openssl-1.1.1g \
    && apt-get -y autoremove && apt-get -y clean && rm -rf /var/lib/apt \
    && rm -rf /tmp && mkdir /tmp && chmod 777 /tmp \
    && rm -rf /usr/share/doc /usr/share/man /usr/share/locale

ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig:/usr/arm-linux-gnueabihf/lib/pkgconfig \
    CC_arm_unknown_linux_gnueabihf=/usr/arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc \
    CXX_arm_unknown_linux_gnueabihf=/usr/arm-linux-gnueabihf/bin/arm-linux-gnueabihf-g++ \
    CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_LINKER=/usr/arm-linux-gnueabihf/bin/arm-linux-gnueabihf-gcc
