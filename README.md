# [CEF](https://bitbucket.org/chromiumembedded/cef) (Chromium Embedded Framework) in [ClassiCube](https://www.classicube.net/)

## Compiling

- [Rust](https://www.rust-lang.org/) nightly
- Download CEF "Standard Distribution"
  - Must be version `cef_binary_81.2.17+gb382c62+chromium-81.0.4044.113`
  - http://opensource.spotify.com/cefbuilds/index.html
  - Extract and rename the `cef_binary_...` folder to `cef_interface/cef_binary`

* Run `cargo build --release`

## Installing

- Copy these files to the same folder as `ClassiCube.exe`:
  - All files in `cef_binary/Release/`
  - All files in `cef_binary/Resources/`
  - `target/release/build/classicube-cef-plugin-*/out/cefsimple`
    - there will be 2 folders named `classicube-cef-plugin-*`, look in both to find `out`
    - rename to "cefsimple" if it had a x86_64/etc suffix

* Copy our compiled plugin from `target/release/classicube_cef_plugin.dll` to `ClassiCube/plugins/`

### Running on Linux

```
cd classicube
export LD_LIBRARY_PATH=.
export PATH=$PATH:.
./ClassiCube
```
