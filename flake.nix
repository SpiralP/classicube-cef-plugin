{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";
    nixpkgs-mozilla.url = "github:mozilla/nixpkgs-mozilla/master";
  };

  outputs = { nixpkgs, nixpkgs-mozilla, ... }:
    let
      inherit (nixpkgs) lib;

      makePackage = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ nixpkgs-mozilla.overlays.rust ];
          };

          rustPlatform =
            let
              rust = (pkgs.rustChannelOf {
                channel = "1.73.0";
                sha256 = "sha256-rLP8+fTxnPHoR96ZJiCa/5Ans1OojI7MLsmSqR2ip8o=";
              }).rust.override {
                extensions = if dev then [ "rust-src" ] else [ ];
              };
            in
            pkgs.makeRustPlatform {
              cargo = rust;
              rustc = rust;
            };
        in
        rustPlatform.buildRustPackage {
          name = "classicube-cef-plugin";
          src =
            let
              cef_binary = pkgs.fetchzip {
                url = "https://cef-builds.spotifycdn.com/cef_binary_117.2.5%2Bgda4c36a%2Bchromium-117.0.5938.152_linux64.tar.bz2";
                hash = "sha256-9+4XRnbRbI22VMa/7CftXLbFQHHsKgDX4kJS7TCoR94=";
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

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "async-dispatcher-0.1.0" = "sha256-rqpQ176/PnI9vvPrwQvK3GJbryjb3hHkb+o1RyCZ3Vg=";
              "clap-4.2.7" = "sha256-Ijwpk9tDIxQVYPE8t4wI1RS9CyhxB/UC5MVD9jnsXGc=";
              "classicube-helpers-2.0.0+classicube.1.3.6" = "sha256-yUl0B0E8P618S0662u70zUGRAG2bETVmb4G7Tbv+ZP4=";
              "classicube-sys-3.0.0+classicube.1.3.6" = "sha256-algb9pgkJdXaswcB6m8DITzORGtOQkSgkhVvwgNXAhI=";
            };
          };

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
            nspr

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

          # TODO ld: warning: libgobject-2.0.so.0, needed by cef_interface/cef_binary/Release/libcef.so, not found
          doCheck = false;
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system}.default = makePackage system true;
        packages.${system}.default = makePackage system false;
      })
      lib.systems.flakeExposed);
}
