#!/usr/bin/env fish

set regex '\d+\.\d+\.\d+\+\w+\+chromium-\d+\.\d+\.\d+\.\d+'
set regex_encoded (string replace --all '\\+' '%2B' $regex)

set from_encoded (cat .github/workflows/rust.yml | rg -o $regex_encoded |head -n1)
set from (string replace --all '%2B' '+' $from_encoded)

set to $argv
set to_encoded (string replace --all '+' '%2B' $to)


sd --fixed-strings $from_encoded $to_encoded .github/workflows/rust.yml
sd --fixed-strings $from $to flake.nix
set sha256 (nix-prefetch-url --unpack --type sha256 --name "cef_binary-$to" "https://cef-builds.spotifycdn.com/cef_binary_$to_encoded""_linux64.tar.bz2")
set hash sha256-(nix-hash --to-base64 --type sha256 $sha256)
echo "$hash"
sd 'hash = "sha256-.+";' "hash = \"$hash\";" flake.nix
