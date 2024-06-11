{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      makePackages = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
          inherit (lib.importTOML ./Cargo.toml) package;

          cef_binary = pkgs.stdenv.mkDerivation rec {
            pname = "cef_binary";
            version = "125.0.21+gc8b1a8c+chromium-125.0.6422.142";

            src = pkgs.fetchzip {
              name = "cef_binary-${version}";
              url = "https://cef-builds.spotifycdn.com/cef_binary_${version}_linux64.tar.bz2";
              hash = "sha256-g0V7PhXo0fd/bk2OWVa7NYd/oQMmfdQRwRNttqbD2vs=";
            };

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

            buildPhase = ''
              patchelf \
                --add-rpath "${lib.makeLibraryPath buildInputs}" \
                Release/*.so Debug/*.so
            '';

            installPhase = ''
              mkdir -v $out
              mv -v * $out/
            '';
          };

          makePackage = (cef_debug:
            let
              cef_profile = if cef_debug then "Debug" else "Release";
            in
            pkgs.rustPlatform.buildRustPackage {
              pname = package.name;
              version = package.version;

              src = lib.sourceByRegex ./. [
                "^\.cargo(/.*)?$"
                "^build\.rs$"
                "^Cargo\.(lock|toml)$"
                "^cef_interface(/.*)?$"
                "^src(/.*)?$"
              ];

              cargoLock = {
                lockFile = ./Cargo.lock;
                outputHashes = {
                  "async-dispatcher-0.1.0" = "sha256-rqpQ176/PnI9vvPrwQvK3GJbryjb3hHkb+o1RyCZ3Vg=";
                  "clap-4.2.7" = "sha256-PccqMT2KltTC2gVL9/xfCNFOAu3+6ash9HqM/TkpgmU=";
                  "classicube-helpers-2.0.0+classicube.1.3.6" = "sha256-V5PBZR0rj42crA1fGUjMk4rDh0ZpjjNcbMCe6bgotW8=";
                };
              };

              nativeBuildInputs = with pkgs; [
                cmake
                pkg-config
                rustPlatform.bindgenHook
              ] ++ (if dev then
                with pkgs; ([
                  cargo-release
                  clippy
                  rustfmt
                  rust-analyzer
                ]) else [ ]);

              ZSTD_SYS_USE_PKG_CONFIG = "1";
              OPENSSL_LIB_DIR = "${lib.getLib pkgs.openssl}/lib";
              OPENSSL_INCLUDE_DIR = "${lib.getDev pkgs.openssl}/include";
              buildInputs = with pkgs; [
                cef_binary
                openssl
                zstd
              ]
              ++ cef_binary.buildInputs;

              postPatch =
                if cef_debug then ''
                  substituteInPlace build.rs \
                    --replace 'let profile = "Release";' 'let profile = "Debug";'
                '' else "";

              preBuild = ''
                chmod -c u+w cef_interface
                cp -va ${cef_binary} cef_interface/cef_binary
              '';

              dontUseCargoParallelTests = true;
              checkPhase = ''
                LD_LIBRARY_PATH=./cef_interface/cef_binary/${cef_profile} cargoCheckHook
              '';

              postInstall = ''
                install -Dm755 ./target/${pkgs.rust.toRustTargetSpec pkgs.stdenv.hostPlatform}/release/build/classicube-cef-plugin-*/out/cef -t $out/bin
              '';

              postFixup = ''
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
            });
        in
        {
          inherit cef_binary;

          default = makePackage false;
          debug = makePackage true;
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system} = makePackages system true;
        packages.${system} = makePackages system false;
      })
      lib.systems.flakeExposed);
}
