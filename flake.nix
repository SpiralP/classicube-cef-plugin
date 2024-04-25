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

          cef_binary = pkgs.libcef.overrideAttrs (prev: rec {
            version = "124.3.6+g30772e7+chromium-124.0.6367.119";

            src = pkgs.fetchzip {
              name = "cef_binary-${version}";
              url = "https://cef-builds.spotifycdn.com/cef_binary_${version}_linux64.tar.bz2";
              hash = "sha256-w9ctBOZujlgqjqfsVedeNDY+IA0p+1xi+jTZjNwDzAA=";
            };

            installPhase =
              let
                prevInstallPhase = pkgs.runCommand "prevInstallPhase"
                  {
                    prevInstallPhase = prev.installPhase;
                    passAsFile = (prev.passAsFile or [ ]) ++ [ "prevInstallPhase" ];
                  }
                  ''
                    substituteInPlace "$prevInstallPhasePath" \
                      --replace '/Release/libcef.so' '/Release/*.so' \
                      --replace 'out/lib/libcef.so' 'out/lib/*.so'
                    cat "$prevInstallPhasePath" > $out
                  '';

                rpathAppend = with pkgs; with xorg; lib.makeLibraryPath [
                  libudev0-shim
                ];
              in
              ''
                ${builtins.readFile prevInstallPhase} 
                # cef wants icu file next to the .so
                mv -v $out/share/cef/* $out/lib/
                rmdir $out/share/cef $out/share

                # needed to fix:
                # - FATAL:udev_loader.cc(37)] Check failed: false.
                # - ERROR:zygote_linux.cc(625)] Zygote could not fork: process_type gpu-process numfds 3 child_pid -1
                patchelf --add-rpath "${rpathAppend}" $out/lib/*.so
              '';

            passthru = {
              debug = pkgs.libcef.overrideAttrs (prev: {
                inherit version src;

                pname = "${prev.pname}-debug";

                hardeningDisable = [ "fortify" ];
                cmakeBuildType = "Debug";

                installPhase = builtins.replaceStrings [ "/Release/" ] [ "/Debug/" ] installPhase;
              });
            };
          });

          makePackage = (cef_debug:
            let
              cef_profile = if cef_debug then "Debug" else "Release";
            in
            pkgs.rustPlatform.buildRustPackage rec {
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

              buildInputs = with pkgs; [
                # gdk-pixbuf
                # gtk3
                cef_binary
                openssl

                # needed to fix "FATAL:udev_loader.cc(37)] Check failed: false."
                # libudev0-shim
              ];

              # TODO profiles
              # postPatch =
              #   if cef_debug then ''
              #     substituteInPlace build.rs \
              #       --replace 'let profile = "Release";' 'let profile = "Debug";'
              #   '' else "";

              dontUseCargoParallelTests = true;
              # TODO
              # checkPhase = ''
              #   LD_LIBRARY_PATH=./cef_interface/cef_binary/${cef_profile} cargoCheckHook
              # '';

              postInstall = ''
                install -Dm755 ./target/${pkgs.rust.toRustTargetSpec pkgs.stdenv.hostPlatform}/release/build/classicube-cef-plugin-*/out/cef -t $out/bin
              '';

              postFixup = ''
                mv -v $out/lib $out/plugins
                mv -v $out/bin $out/cef

                mkdir -vp $out/cef/cef_binary
                ln -vs ${cef_binary}/lib/* $out/cef/cef_binary/

                # TODO
                # patchelf --debug \
                #   --add-rpath "\$ORIGIN/../cef/cef_binary" \
                #   $out/plugins/libclassicube_cef_plugin.so \
                #   $out/cef/cef
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
