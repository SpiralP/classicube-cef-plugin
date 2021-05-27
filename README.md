# [CEF](https://bitbucket.org/chromiumembedded/cef) (Chromium Embedded Framework) in [ClassiCube](https://www.classicube.net/)

A ClassiCube plugin that allows placing web browser screens in-game!

![image](https://i.imgur.com/MyvxVCZ.png)

**You probably want [the loader plugin](https://github.com/SpiralP/classicube-cef-loader-plugin/blob/master/README.md#installing) that installs and updates this plugin instead of compiling it yourself!**

## Prerequisites

- [Rust](https://www.rust-lang.org/) **nightly**
- Some other stuff; you can look in [the GitHub Actions script](.github/workflows/rust.yml) for platform-specific dependencies

## Compiling

- Clone this repo

* Download CEF "Standard Distribution"
  - https://cef-builds.spotifycdn.com/index.html
  - Extract and rename the `cef_binary_...` folder to `cef_interface/cef_binary`

- Run `cargo build --release`
  - This will create:
    - `target/release/classicube_cef_plugin.dll`
    - `target/release/build/classicube-cef-plugin-*/out/cef.exe`
      - There will be 2 folders named `classicube-cef-plugin-*`, look in both to find `out`

## Installing

- Copy these files to the same folder as the `ClassiCube` executable:
  - All files in `cef_binary/Release/`
  - All files in `cef_binary/Resources/`
  - Our `cef.exe` file
    - Rename this to `cef.exe` if it wasn't already

* Copy our plugin `classicube_cef_plugin.dll` to `ClassiCube/plugins/`
