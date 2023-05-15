{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
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
                channel = "1.69.0";
                sha256 = "sha256-eMJethw5ZLrJHmoN2/l0bIyQjoTX1NsvalWSscTixpI=";
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

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "async-dispatcher-0.1.0" = "sha256-rqpQ176/PnI9vvPrwQvK3GJbryjb3hHkb+o1RyCZ3Vg=";
              "clap-2.33.0" = "sha256-o8m9H2V0J5ID9i4eAR4oEN3yxWRhUQ4WljUEk8EU74g=";
              "classicube-helpers-2.0.0+classicube.1.3.5" = "sha256-kPvJERlyoUbk8NsyWauwJcsObwAWLVjyuPO/n4LQXoc=";
              "classicube-sys-2.0.0+classicube.1.3.5" = "sha256-VXHyJwF8cdX3PlTn7xgbviGg3D5bsRRh375I+DRpE4g=";
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
