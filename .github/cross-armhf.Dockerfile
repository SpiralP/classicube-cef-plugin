# docker build -t cross-armhf -f cross-armhf.Dockerfile .
# cross build --target armv7-unknown-linux-gnueabihf --no-default-features

# ubuntu-like
FROM rustembedded/cross:armv7-unknown-linux-gnueabihf-0.2.1

# we're root
RUN dpkg --add-architecture armhf \
    && apt-get -y update \
    && apt-get -y install git gcc g++ build-essential cmake clang-8 pkg-config \
    gcc-arm-linux-gnueabihf g++-arm-linux-gnueabihf libc6-dev-i386 \
    libssl-dev:armhf libglib2.0-dev:armhf libpango1.0-dev:armhf libatk1.0-dev:armhf libgtk-3-dev:armhf libgdk-pixbuf2.0-dev:armhf \
    libnss3:armhf libasound2:armhf libxss1:armhf libnspr4:armhf \
    # cleanup
    && apt-get -y autoremove && apt-get -y clean && rm -rf /var/lib/apt \
    && rm -rf /tmp && mkdir /tmp && chmod 777 /tmp \
    && rm -rf /usr/share/doc /usr/share/man /usr/share/locale

ENV PKG_CONFIG_PATH=/usr/lib/arm-linux-gnueabihf/pkgconfig
