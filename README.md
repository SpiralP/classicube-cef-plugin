# [CEF](https://bitbucket.org/chromiumembedded/cef) (Chromium Embedded Framework) in [ClassiCube](https://www.classicube.net/)

## Compiling

- Rust nightly
- Download CEF "Standard Distribution"
  - http://opensource.spotify.com/cefbuilds/index.html#windows64_builds
  - Extract and rename the `cef_binary_...` folder to `cef_interface/cef_binary`

* Run `cargo build --release`

## Installing

- Copy these files to the same folder as `ClassiCube.exe`:
  - All files in `cef_binary/Release/`
  - All files in `cef_binary/Resources/`
  - `target/release/build/classicube-cef-plugin-*/out/cefsimple.exe`
    - there will be 2 folders named `classicube-cef-plugin-*`, look in both to find `out`

* Copy our compiled plugin from `target/release/classicube_cef_plugin.dll` to `ClassiCube/plugins/`
