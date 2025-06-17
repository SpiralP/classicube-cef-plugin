{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      makePackages = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        rec {
          update-cef-version = pkgs.writeShellApplication {
            name = "update-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
            ];
            text = ''
              LATEST_VERSION="$(${lib.getExe get-latest-cef-version})"
              CURRENT_VERSION="$(${lib.getExe get-current-cef-version})"

              test -z "$LATEST_VERSION" && exit 1
              test -z "$CURRENT_VERSION" && exit 1

              if test "$LATEST_VERSION" != "$CURRENT_VERSION"; then
                echo "new CEF version: $LATEST_VERSION"
                ${lib.getExe replace-cef-version} "$LATEST_VERSION"
              else
                echo "already at latest version: $LATEST_VERSION"
              fi
            '';
          };

          get-latest-cef-version = pkgs.writeShellApplication {
            name = "get-latest-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
              zx
            ];
            text = ''
              LATEST_VERSION="$(zx ${./get-latest-cef-version.mjs})"
              test -z "$LATEST_VERSION" && exit 1
              echo "$LATEST_VERSION"
            '';
          };

          get-current-cef-version = pkgs.writeShellApplication {
            name = "get-current-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
            ];
            text = ''
              CURRENT_VERSION="$(cat cef_binary_version)"
              test -z "$CURRENT_VERSION" && exit 1
              echo "$CURRENT_VERSION"
            '';
          };

          replace-cef-version = pkgs.writeShellApplication {
            name = "replace-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
              gnugrep
              nix
              sd
            ];
            text = ''
              NEW_VERSION="$1"
              OLD_VERSION="$(${lib.getExe get-current-cef-version})"

              echo "$OLD_VERSION" "$NEW_VERSION"
              test -z "$OLD_VERSION" && echo 'OLD_VERSION missing' && exit 1
              test -z "$NEW_VERSION" && echo 'NEW_VERSION missing' && exit 1
              test "$OLD_VERSION" = "$NEW_VERSION" && exit 0
              
              echo "$NEW_VERSION" > cef_binary_version

              URL="$(nix eval --no-update-lock-file --raw .#cef_binary.src.url)"
              NAME="$(nix eval --no-update-lock-file --raw .#cef_binary.src.name)"
              OLD_HASH="$(nix eval --no-update-lock-file --raw .#cef_binary.src.outputHash)"
              NEW_SHA256="$(nix-prefetch-url --unpack --type sha256 --name "$NAME" "$URL")"
              NEW_HASH="sha256-$(nix-hash --to-base64 --type sha256 "$NEW_SHA256")"
              echo "$OLD_HASH" "$NEW_HASH"

              if ! grep -q "$OLD_HASH" flake.nix; then
                echo "couldn't find old hash in flake.nix"
                exit 1
              fi
              sd --fixed-strings "$OLD_HASH" "$NEW_HASH" flake.nix
              if ! grep -q "$NEW_HASH" flake.nix; then
                echo "couldn't find new hash in flake.nix"
                exit 1
              fi
            '';
          };
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
