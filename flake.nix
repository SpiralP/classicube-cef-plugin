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

              platforms."x86_64-linux".hash = "sha256-O1q8b/zlUB+t1ZQbbF4S+ZNh1BrVF3NS21r6e98/34E=";
              # platforms."aarch64-linux".hash = "";
              # platforms."armv7l-linux".hash = "";
              # platforms."x86_64-darwin".hash = "";
              # platforms."aarch64-darwin".hash = "";

              inherit (platforms.${pkgs.stdenv.hostPlatform.system}) platformUrl projectArchCmake hash;
            in
            (debug: prev: (rec {
              version = lib.strings.trim (builtins.readFile ./cef_binary_version);

              src = pkgs.fetchzip {
                inherit hash;
                name = "cef_binary-${version}";
                url = "https://cef-builds.spotifycdn.com/cef_binary_${version}_${platformUrl}.tar.bz2";
              };

              installPhase = ''
                pwd
                cd ..
                pwd
                find .

                # conserve disk space, before installPhase runs cp
                rm -rfv ${if debug then "Release" else "Debug"}

                ${prev.installPhase}

                # cef wants icu file next to the .so
                mv -vT $out/${if debug then "Debug" else "Release"} $out/lib
                mv -v $out/Resources/* $out/lib/
                mv -v $out/build/libcef_dll_wrapper/libcef_dll_wrapper.a $out/lib/

                find $out -mindepth 1 -maxdepth 1 ! -name lib ! -name include -exec rm -rfv {} +
              '';

              nativeBuildInputs = with pkgs; [ cmake ];

              makeFlags = [ "libcef_dll_wrapper" ];

              cmakeFlags =
                if builtins.length (prev.cmakeFlags or [ ]) == 0
                then [ "-DPROJECT_ARCH=${projectArchCmake}" ]
                else throw "cmakeFlags changed?";

              meta = prev.meta // {
                platforms = builtins.attrNames platforms;
              };
            }) // (
              lib.attrsets.optionalAttrs debug {
                pname = "${prev.pname}-debug";

                cmakeBuildType = "Debug";

                hardeningDisable = [ "all" ];
              }
            ));

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

          cef_binary = pkgs.cef-binary.overrideAttrs (makeCefBinaryAttrs false);

          cef_binary_debug = (pkgs.enableDebugging pkgs.cef-binary).overrideAttrs (makeCefBinaryAttrs true);
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
