{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;

      rustManifest = lib.importTOML ./Cargo.toml;

      revSuffix = lib.optionalString (self ? dirtyShortRev)
        "-${self.dirtyShortRev}";

      makePackages = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
          };

          makeCefBinaryAttrs =
            let
              platforms = {
                "x86_64-linux" = { platformUrl = "linux64"; projectArchCmake = "x86_64"; };
                # TODO test if arm builds/works before adding these
                # "aarch64-linux" = { platformUrl = "linuxarm64"; projectArchCmake = "arm64"; };
                # "armv7l-linux" = { platformUrl = "linuxarm"; projectArchCmake = "arm"; };
                # TODO need to remove unsupported rpath's on libcef
                # "x86_64-darwin" = { platformUrl = "macosx64"; projectArchCmake = "x86_64"; };
                # "aarch64-darwin" = { platformUrl = "macosarm64"; projectArchCmake = "arm64"; };
              };

              platforms."x86_64-linux".hash = "sha256-ZMi93R6Bp9kYmGBob8WYCNHsHHrvAreMBB6nNTwLVIQ=";
              # platforms."aarch64-linux".hash = "";
              # platforms."armv7l-linux".hash = "";
              # platforms."x86_64-darwin".hash = "";
              # platforms."aarch64-darwin".hash = "";

              inherit (platforms.${pkgs.stdenv.hostPlatform.system}) platformUrl projectArchCmake hash;
            in
            (prev: rec {
              version = lib.strings.trim (builtins.readFile ./cef_binary_version);

              src = pkgs.fetchzip {
                inherit hash;
                name = "cef_binary-${version}";
                url = "https://cef-builds.spotifycdn.com/cef_binary_${version}_${platformUrl}.tar.bz2";
              };

              installPhase = prev.installPhase + ''
                # cef wants icu file next to the .so
                mv -v $out/share/cef/* $out/lib/
                rmdir $out/share/cef $out/share

                # old: needed to fix "FATAL:udev_loader.cc(48)] Check failed: false."
                # needs libudev.so.1 now instead of previous ^ so.0 to link at compile time
                patchelf --add-rpath "${lib.makeLibraryPath [ pkgs.udev ]}" $out/lib/*.so
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
            pname = rustManifest.package.name;
            version = rustManifest.package.version + revSuffix;

            src = lib.sourceByRegex ./. [
              "^\.cargo(/.*)?$"
              "^build\.rs$"
              "^Cargo\.(lock|toml)$"
              "^cef_interface(/.*)?$"
              "^src(/.*)?$"
            ];

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            nativeBuildInputs = with pkgs; [
              cmake
              pkg-config
              rustPlatform.bindgenHook
            ] ++ (if dev then
              with pkgs; ([
                cargo-release
                clippy
                (rustfmt.override { asNightly = true; })
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

            # so that when developing we don't get spammed with
            # "warning _FORTIFY_SOURCE requires compiling with optimization (-O)"
            hardeningDisable =
              if dev
              then [ "all" ]
              else [ ];
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
