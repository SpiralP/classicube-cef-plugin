{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    flake-utils.url = "github:SpiralP/nix-flake-utils";
  };

  outputs = inputs@{ flake-utils, ... }:
    flake-utils.lib.makeOutputs inputs
      ({ lib, pkgs, system, dev, makeRustPackage, ... }:
        let
          platforms = {
            "x86_64-linux" = {
              platformUrl = "linux64";
              projectArchCmake = "x86_64";
              hash = "sha256-kBfUz9jrBxLbELvgrU2gbgEDC1z0y2jkhhdt7QJY4ZE=";
            };
            # TODO test if arm builds/works before adding these
            # "aarch64-linux" = { platformUrl = "linuxarm64"; projectArchCmake = "arm64"; hash = ""; };
            # "armv7l-linux"  = { platformUrl = "linuxarm";   projectArchCmake = "arm";   hash = ""; };
            # TODO need to remove unsupported rpath's on libcef
            # "x86_64-darwin"  = { platformUrl = "macosx64";   projectArchCmake = "x86_64"; hash = ""; };
            # "aarch64-darwin" = { platformUrl = "macosarm64"; projectArchCmake = "arm64";  hash = ""; };
          };

          supported = builtins.hasAttr system platforms;

          makeCefBinaryAttrs = debug: prev:
            let
              p = platforms.${system};
            in
            (rec {
              version = lib.strings.trim (builtins.readFile ./cef_binary_version);

              src = pkgs.fetchzip {
                inherit (p) hash;
                name = "cef_binary-${version}";
                url = "https://cef-builds.spotifycdn.com/cef_binary_${version}_${p.platformUrl}.tar.bz2";
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
                then [ "-DPROJECT_ARCH=${p.projectArchCmake}" ]
                else throw "cmakeFlags changed?";

              meta = prev.meta // {
                platforms = builtins.attrNames platforms;
              };
            }) // lib.attrsets.optionalAttrs debug {
              pname = "${prev.pname}-debug";
              cmakeBuildType = "Debug";
              hardeningDisable = [ "all" ];
            };

          cef_binary = pkgs.cef-binary.overrideAttrs (makeCefBinaryAttrs false);
          cef_binary_debug = pkgs.cef-binary.overrideAttrs (makeCefBinaryAttrs true);

          src = lib.sourceByRegex ./. [
            "^\.cargo(/.*)?$"
            "^build\.rs$"
            "^Cargo\.(lock|toml)$"
            "^cef_interface(/.*)?$"
            "^src(/.*)?$"
          ];

          makeArgs = cb: {
            inherit src;

            nativeBuildInputs = with pkgs; [
              cmake
              pkg-config
              rustPlatform.bindgenHook
            ];

            buildInputs = with pkgs; [ cb openssl zstd ];

            LIBCEF_LIB_DIR = "${cb}/lib";
            LIBCEF_INCLUDE_DIR = "${cb}/include";
            LIBCEF_DLL_WRAPPER_LIB_DIR = "${cb}/lib";

            ZSTD_SYS_USE_PKG_CONFIG = "1";
            OPENSSL_LIB_DIR = "${lib.getLib pkgs.openssl}/lib";
            OPENSSL_INCLUDE_DIR = "${lib.getDev pkgs.openssl}/include";

            dontUseCargoParallelTests = true;
            useNextest = true;

            postFixup = ''
              mv -v $out/lib $out/plugins
              mv -v $out/bin $out/cef

              mkdir -vp $out/cef/cef_binary
              ln -vs ${cb}/lib/* $out/cef/cef_binary/
            '';

            # so that when developing we don't get spammed with
            # "warning _FORTIFY_SOURCE requires compiling with optimization (-O)"
            hardeningDisable = lib.optionals dev [ "all" ];
          };
        in
        lib.optionalAttrs supported {
          default = makeRustPackage pkgs (makeArgs cef_binary);

          debug = makeRustPackage pkgs (makeArgs cef_binary_debug // {
            buildType = "debug";
            hardeningDisable = [ "all" ];
          });

          inherit cef_binary cef_binary_debug;
        });
}
