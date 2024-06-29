{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
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


          makeCefBinaryAttrs =
            let
              platforms = {
                "x86_64-linux" = { platformUrl = "linux64"; projectArchCmake = "x86_64"; };
                "aarch64-linux" = { platformUrl = "linuxarm64"; projectArchCmake = "arm64"; };
                "armv7l-linux" = { platformUrl = "linuxarm"; projectArchCmake = "arm"; };
                # TODO need to remove unsupported rpath's on libcef
                "x86_64-darwin" = { platformUrl = "macosx64"; projectArchCmake = "x86_64"; };
                "aarch64-darwin" = { platformUrl = "macosarm64"; projectArchCmake = "arm64"; };
              };

              platforms."x86_64-linux".hash = "sha256-/5FrvF6YuUbU1vLft+eLdTqUJs5b+ABfpxSGdh0WztA=";
              platforms."aarch64-linux".hash = "sha256-NbIdkajKoSqbVgQney3RfyBlkr3xGKyRQTw6Sca9EOo=";
              platforms."armv7l-linux".hash = "sha256-1FQXmh9E+wY/+WjR94znJkDNwFTesLA8XxJZoKIA4PA=";
              platforms."x86_64-darwin".hash = "sha256-QWrkZoOuMFH9m8eTfnhpg6avC8ZELzH9uor/0zmVsVk=";
              platforms."aarch64-darwin".hash = "";

              inherit (platforms.${pkgs.stdenv.hostPlatform.system}) platformUrl projectArchCmake hash;
            in
            (prev: rec {
              version = "126.2.9+g169fea9+chromium-126.0.6478.127";

              src = pkgs.fetchzip {
                inherit hash;
                name = "cef_binary-${version}";
                url = "https://cef-builds.spotifycdn.com/cef_binary_${version}_${platformUrl}.tar.bz2";
              };

              installPhase = ''
                ${prev.installPhase}

                # cef wants icu file next to the .so
                mv -v $out/share/cef/* $out/lib/
                rmdir $out/share/cef $out/share

                # needed to fix "FATAL:udev_loader.cc(48)] Check failed: false."
                patchelf --add-rpath "${lib.makeLibraryPath [pkgs.libudev0-shim]}" $out/lib/*.so
              '';

              cmakeFlags =
                if builtins.length prev.cmakeFlags == 1
                then [ "-DPROJECT_ARCH=${projectArchCmake}" ]
                else throw "cmakeFlags changed?";

              meta = prev.meta // {
                platforms = builtins.attrNames platforms;
              };
            });

          makeDefaultAttrs = (cef_binary: rec {
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

            LIBCEF_LIB_DIR = "${cef_binary}/lib";
            LIBCEF_INCLUDE_DIR = "${cef_binary}/include";
            LIBCEF_DLL_WRAPPER_LIB_DIR = LIBCEF_LIB_DIR;

            ZSTD_SYS_USE_PKG_CONFIG = "1";
            OPENSSL_LIB_DIR = "${lib.getLib pkgs.openssl}/lib";
            OPENSSL_INCLUDE_DIR = "${lib.getDev pkgs.openssl}/include";
            buildInputs = with pkgs; [
              cef_binary
              openssl
              zstd
            ];

            dontUseCargoParallelTests = true;

            postFixup = ''
              mv -v $out/lib $out/plugins
              mv -v $out/bin $out/cef

              mkdir -vp $out/cef/cef_binary
              ln -vs ${cef_binary}/lib/* $out/cef/cef_binary/
            '';
          });
        in
        rec {
          default = pkgs.rustPlatform.buildRustPackage (makeDefaultAttrs cef_binary);

          debug = (pkgs.enableDebugging {
            inherit (pkgs) stdenv;
            override = (attrs: pkgs.makeRustPlatform ({
              inherit (pkgs) rustc cargo;
            } // attrs));
          }).buildRustPackage (
            let
              attrs = makeDefaultAttrs cef_binary_debug;
            in
            (attrs // {
              pname = "${attrs.pname}-debug";

              buildType = "debug";

              hardeningDisable = [ "all" ];
            })
          );

          cef_binary = pkgs.libcef.overrideAttrs makeCefBinaryAttrs;

          cef_binary_debug = (pkgs.enableDebugging pkgs.libcef).overrideAttrs (prev:
            let
              attrs = makeCefBinaryAttrs prev;
            in
            attrs // {
              pname = "${prev.pname}-debug";

              cmakeBuildType = "Debug";

              installPhase = builtins.replaceStrings [ "/Release/" ] [ "/Debug/" ] attrs.installPhase;

              hardeningDisable = [ "all" ];
            });
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system} = makePackages system true;
        packages.${system} = makePackages system false;
      })
      [
        "x86_64-linux"
        # TODO
        "aarch64-linux"
        # TODO
        # cef "Linux ARM" is armv7
        "armv7l-linux"

        # TODO
        "x86_64-darwin"
        # TODO
        "aarch64-darwin"
      ]);
}
