{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
    nixpkgs-mozilla.url = "github:mozilla/nixpkgs-mozilla/master";
  };

  outputs = { nixpkgs, nixpkgs-mozilla, ... }:
    let
      inherit (nixpkgs) lib;
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ nixpkgs-mozilla.overlays.rust ];
          };

          rustPlatform =
            let
              rust = (pkgs.rustChannelOf {
                date = "2023-04-23";
                channel = "nightly";
                sha256 = "sha256-f+dMK7oRvMx2VYzqJru4ElIngARn4d2q2GkAPdlZrW0=";
              }).rust.override {
                extensions = [ "rust-src" ];
              };
            in
            pkgs.makeRustPlatform {
              cargo = rust;
              rustc = rust;
            };

          package = rustPlatform.buildRustPackage {
            name = "classicube-cef-plugin";
            src =
              let
                cef_binary = pkgs.fetchzip {
                  url = "https://cef-builds.spotifycdn.com/cef_binary_112.3.0%2Bgb09c4ca%2Bchromium-112.0.5615.165_linux64.tar.bz2";
                  sha256 = "sha256-0Ehf8u9aFtl9i7Rt7+Qtm7UUIBO15VaRbUs3cxtg3kk=";
                };

                code = lib.cleanSourceWith rec {
                  src = ./.;
                  filter = path: type:
                    lib.cleanSourceFilter path type
                    && (
                      let
                        baseName = builtins.baseNameOf (builtins.toString path);
                        relPath = lib.removePrefix (builtins.toString ./.) (builtins.toString path);
                      in
                      lib.any (re: builtins.match re relPath != null) [
                        "/build.rs"
                        "/Cargo.toml"
                        "/Cargo.lock"
                        "/\.cargo"
                        "/\.cargo/.*"
                        "/cef_interface"
                        "/cef_interface/.*"
                        "/src"
                        "/src/.*"
                      ]
                    );
                };
              in
              pkgs.runCommand "src" { } ''
                cp -a ${code} $out \
                  && chmod +w $out/cef_interface \
                  && ln -s ${cef_binary}/ $out/cef_interface/cef_binary
              '';
            cargoSha256 = "sha256-YMkd0ZheDtMBG+gAs7gVMl+V4tjPpJvisEFdvmebXlQ=";
            nativeBuildInputs = with pkgs; [
              cmake
              pkg-config
              rustPlatform.bindgenHook
            ];
            buildInputs = with pkgs; [
              # things found on libcef.so that were missing
              glib
              nss
              at-spi2-atk
              cups
              libdrm
              xorg.libXcomposite
              xorg.libXdamage
              xorg.libXrandr
              xorg.libXext
              xorg.libXfixes
              mesa
              expat
              xorg.libxcb
              libxkbcommon
              dbus
              pango
              cairo
              alsa-lib

              gdk-pixbuf
              gtk3
              openssl
            ];

            postBuild = with pkgs; ''
              mkdir -p $out/bin \
                && cp -va ./target/${rust.toRustTargetSpec stdenv.hostPlatform}/release/build/classicube-cef-plugin-*/out/cef $out/bin/cef
            '';

            # TODO need to `patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 cef`
            # for a normal linux

            doCheck = false;
          };
        in
        rec {
          devShells.${system}.default = package.overrideAttrs (old: {
            nativeBuildInputs = with pkgs; old.nativeBuildInputs ++ [
              clippy
              rustfmt
              rust-analyzer
            ];
          });
          packages.${system}.default = package;
        }
      )
      lib.systems.flakeExposed);
}
