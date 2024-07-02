# [CEF](https://bitbucket.org/chromiumembedded/cef) (Chromium Embedded Framework) in [ClassiCube](https://www.classicube.net/)

A ClassiCube plugin that allows placing web browser screens in-game!

![image](https://i.imgur.com/MyvxVCZ.png)

**You probably want [the loader plugin](https://github.com/SpiralP/classicube-cef-loader-plugin/blob/master/README.md#installing) that installs and updates this plugin instead of compiling it yourself!**

## Prerequisites

- [Rust](https://www.rust-lang.org/) stable
- Some other stuff; you can look in [the GitHub Actions script](.github/workflows/build.yml) for platform-specific dependencies

## Setup

- Clone this repo

* Download CEF "Standard Distribution"
  - https://cef-builds.spotifycdn.com/index.html
  - Extract and rename the `cef_binary_...` folder to `./cef_interface/cef_binary`

## Build (Windows)

- Run `cargo build --release`
  - This will create:
    - `./target/release/classicube_cef_plugin.dll`
    - `./target/release/cef.exe`

## Install (Windows)

In a directory with the `ClassiCube` executable:

- Copy:
  - All files in `./cef_binary/Release/` and `./cef_binary/Resources/` to `./` (same folder as ClassiCube.exe)
  - The `cef.exe` file to `./cef/cef.exe`

* Copy our plugin `classicube_cef_plugin.dll` to `./plugins/classicube_cef_plugin.dll`

## Build (Linux)

- Run `cargo build --release`
  - This will create:
    - `./target/release/libclassicube_cef_plugin.so`
    - `./target/release/cef`

## Install (Linux)

In a directory with the `ClassiCube` executable:

- Copy:
  - All files in `./cef_binary/Release/*` and `./cef_binary/Resources/*` to `./cef/cef_binary/`
  - The `cef` file to `./cef/cef`

* Copy our plugin `libclassicube_cef_plugin.so` to `./plugins/libclassicube_cef_plugin.so`

## Build and Install With ([Nix](https://nixos.org/))

In a directory with the `ClassiCube` executable:

- `nix build github:SpiralP/classicube-cef-plugin`
- `cp -va result/* . && chmod -cR u+w cef plugins`
- if using on another linux os: `patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 ./cef/cef`

## Errors

- Linux: `cannot allocate memory in static TLS block`
  - `GLIBC_TUNABLES=glibc.rtld.optional_static_tls=16384 ./ClassiCube`
- Mac:
  - in order to run ClassiCube with CEF on macOS 11+, you have to run without 10.16 compatibility:
    `SYSTEM_VERSION_COMPAT=0 ./ClassiCube`
