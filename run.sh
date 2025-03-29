#!/usr/bin/env bash
set -eux

export RUSTFLAGS="-L$(nix eval --json nixpkgs#SDL2 | jq -r)/lib"

exec cargo run -- "$@"
