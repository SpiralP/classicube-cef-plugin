# cat .github/cross-podman.Dockerfile | command podman build -t temp --progress plain -
# and command podman run --rm -it --device /dev/fuse --user 0:0 -v "$PWD:$PWD" -w "$PWD" temp cross build --target aarch64-unknown-linux-gnu
# and ls -lAhSr target/aarch64-unknown-linux-gnu/debug/

FROM rust@sha256:4fd8406017c992f7b8ab55a2f99a1d56aeb1d7ecd255850dfa04239a88601f73

RUN apt-get update && apt-get install -y \
    podman \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cross

RUN useradd --create-home --user-group user \
 && printf '%s\n' \
    'root:1:65535' \
    'user:1:999' \
    'user:1001:64535' \
    | tee /etc/subuid /etc/subgid

RUN chmod u-s /usr/bin/newuidmap /usr/bin/newgidmap \
 && setcap cap_setuid+ep /usr/bin/newuidmap \
 && setcap cap_setgid+ep /usr/bin/newgidmap

RUN printf '%s\n' \
    '[containers]' \
    'netns="host"' \
    'utsns="host"' \
    'cgroups="disabled"' \
    'volumes = ["/proc:/proc"]' \
    '[engine]' \
    'cgroup_manager = "cgroupfs"' \
    > /etc/containers/containers.conf

USER user
WORKDIR /home/user
