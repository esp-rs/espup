#!/usr/bin/env bash
set -e
set -o pipefail

for i in {1..100}
do
    cargo test test_xtensa_rust_parse_version -- --nocapture
# curl --request GET \
# --url https://api.github.com/repos/esp-rs/rust-build/releases \
# --header 'authorization: Bearer ${{ secrets.GITHUB_TOKEN }}'
done
