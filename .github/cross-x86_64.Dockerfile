
# docker build --pull -t cross-x86_64 -f .github/cross-x86_64.Dockerfile .
# cross build --target x86_64-unknown-linux-gnu

FROM debian:latest

# we're root
RUN set -ex \
    && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y update \
    && apt-get -y full-upgrade \
    && apt-get -y install \
    wget curl git gcc g++ build-essential cmake clang pkg-config \
    libssl-dev libglib2.0-dev libpango1.0-dev libatk1.0-dev libgtk-3-dev libgdk-pixbuf2.0-dev \
    libssl1.1 libnss3 libasound2 libxss1 libnspr4 \
    && wget -O /tmp/cmake-install.sh 'https://github.com/Kitware/CMake/releases/download/v3.25.0/cmake-3.25.0-linux-x86_64.sh' \
    && sh /tmp/cmake-install.sh --skip-license --prefix=/usr/local \
    && rm /tmp/cmake-install.sh \
    # cleanup
    && apt-get -y autoremove && apt-get -y clean && rm -rf /var/lib/apt \
    && rm -rf /tmp && mkdir /tmp && chmod 777 /tmp \
    && rm -rf /usr/share/doc /usr/share/man /usr/share/locale
