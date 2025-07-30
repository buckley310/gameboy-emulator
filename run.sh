#!/usr/bin/env bash
set -eux

export RUSTFLAGS="-L$(nix eval --json nixpkgs#sdl3.lib | jq -r)/lib"

exec cargo run -- "$@"
