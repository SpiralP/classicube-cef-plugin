# [CEF](https://bitbucket.org/chromiumembedded/cef) (Chromium Embedded Framework) in [ClassiCube](https://www.classicube.net/)

## Prerequisites

- [Rust](https://www.rust-lang.org/) **nightly**
- Some other stuff, you can look in [the GitHub Actions script](.github/workflows/rust.yml) for platform-specific dependencies

## Compiling

- Clone this repo

* Download CEF "Standard Distribution"
  - http://opensource.spotify.com/cefbuilds/index.html
  - Extract and rename the `cef_binary_...` folder to `cef_interface/cef_binary`

- Run `cargo build --release`
  - This will create:
    - `target/release/classicube_cef_plugin.dll`
    - `target/release/build/classicube-cef-plugin-*/out/cefsimple.exe`
      - There will be 2 folders named `classicube-cef-plugin-*`, look in both to find `out`

## Installing

- Copy these files to the same folder as the `ClassiCube` executable:
  - All files in `cef_binary/Release/`
  - All files in `cef_binary/Resources/`
  - Our `cefsimple.exe` file
    - Rename this to `cefsimple.exe` if it wasn't already

* Copy our plugin `classicube_cef_plugin.dll` to `ClassiCube/plugins/`

### Running on Linux

```
cd ClassiCube/
export LD_LIBRARY_PATH=.
export PATH=$PATH:.
./ClassiCube
```
