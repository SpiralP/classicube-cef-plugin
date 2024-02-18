{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    nixpkgs-mozilla.url = "github:mozilla/nixpkgs-mozilla/master";
  };

  outputs = { nixpkgs, nixpkgs-mozilla, ... }:
    let
      inherit (nixpkgs) lib;

      makePackage = (system: dev: cef_debug:
        let
          cef_profile = if cef_debug then "Debug" else "Release";

          pkgs = import nixpkgs {
            inherit system;
            overlays = [ nixpkgs-mozilla.overlays.rust ];
          };

          rust = (pkgs.rustChannelOf {
            channel = "1.75.0";
            sha256 = "sha256-SXRtAuO4IqNOQq+nLbrsDFbVk+3aVA8NNpSZsKlVH/8=";
          }).rust.override {
            extensions = if dev then [ "rust-src" ] else [ ];
          };
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rust;
            rustc = rust;
          };
        in
        rustPlatform.buildRustPackage rec {
          name = "classicube-cef-plugin";
          src =
            let
              cef_binary =
                let
                  version = "121.3.9+g1e0a38f+chromium-121.0.6167.184";
                  version_url = builtins.replaceStrings [ "+" ] [ "%2B" ] version;
                in
                pkgs.fetchzip {
                  name = "cef_binary-${version}";
                  url = "https://cef-builds.spotifycdn.com/cef_binary_${version_url}_linux64.tar.bz2";
                  hash = "sha256-dcqR6GeRWrH2jpVp7pmQLQdfMQiRiHCp84bxBoTJNlA=";
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
              cp -va ${code} $out
              chmod u+w $out/cef_interface
              cp -va ${cef_binary} $out/cef_interface/cef_binary
            '';

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "async-dispatcher-0.1.0" = "sha256-rqpQ176/PnI9vvPrwQvK3GJbryjb3hHkb+o1RyCZ3Vg=";
              "clap-4.2.7" = "sha256-P8Thh4miozjn/0/EMQzB91ZsEVucZAg8XwMDf6D4vP8=";
              "classicube-helpers-2.0.0+classicube.1.3.6" = "sha256-YvXgatSnaM9YJhk0Sx9dVzn0pl6tOUPrm394aj8qk1o=";
            };
          };

          nativeBuildInputs = with pkgs; [
            cmake
            pkg-config
            rustPlatform.bindgenHook
          ];

          buildInputs = with pkgs; with xorg; [
            # things found on libcef.so that were missing
            glib
            nss
            at-spi2-atk
            cups
            libdrm
            libXcomposite
            libXdamage
            libXrandr
            libXext
            libXfixes
            libX11
            mesa
            expat
            libxcb
            libxkbcommon
            dbus
            pango
            cairo
            alsa-lib
            nspr

            gdk-pixbuf
            gtk3
            openssl

            # needed to fix "FATAL:udev_loader.cc(37)] Check failed: false."
            libudev0-shim
          ];

          postPatch = with pkgs; if cef_debug then ''
            substituteInPlace build.rs \
              --replace 'let profile = "Release";' 'let profile = "Debug";'
          '' else "";

          preBuild = ''
            chmod -c u+w cef_interface/cef_binary/${cef_profile}/*.so
            patchelf \
              --add-rpath "${lib.makeLibraryPath buildInputs}" \
              cef_interface/cef_binary/${cef_profile}/*.so
          '';

          dontUseCargoParallelTests = true;
          checkPhase = ''
            LD_LIBRARY_PATH=./cef_interface/cef_binary/${cef_profile} cargoCheckHook
          '';

          postInstall = with pkgs; ''
            install -Dm755 ./target/${pkgs.rust.toRustTargetSpec stdenv.hostPlatform}/release/build/classicube-cef-plugin-*/out/cef -t $out/bin
          '';

          postFixup = with pkgs; ''
            mv -v $out/lib $out/plugins
            mv -v $out/bin $out/cef

            mkdir -vp $out/cef/cef_binary
            cp -va cef_interface/cef_binary/${cef_profile}/* cef_interface/cef_binary/Resources/* $out/cef/cef_binary/

            patchelf --debug \
              --add-rpath "\$ORIGIN/../cef/cef_binary" \
              $out/plugins/libclassicube_cef_plugin.so \
              $out/cef/cef
          '';

          hardeningDisable = if cef_debug then [ "fortify" ] else [ ];
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system}.default = makePackage system true false;
        packages.${system} = {
          default = makePackage system false false;
          debug = makePackage system false true;
        };
      })
      lib.systems.flakeExposed);
}
