#!/usr/bin/env bash
# Runs the workspace tests against the reference stack on Linux (dev/up.sh, dev/seed.sh), with the paths
# the golden tests otherwise take from the Windows development machine:
#   dev/test.sh [cargo test arguments]   e.g. dev/test.sh --release --no-fail-fast
# The authentication oracle (crates/rustpress-auth/tests/oracle.rs) also needs RUSTPRESS_TEST_WP_CONFIG; its
# fixture holds cookies and nonces made with the salts of the installation it was recorded on, so set it only
# there (or record a new fixture with dev/auth_oracle.sh).
set -euo pipefail
cd "$(dirname "$0")/.."
root=${RUSTPRESS_WP_ROOT:-$(podman volume inspect rustpress-wp -f '{{.Mountpoint}}')}
export RUSTPRESS_WP_ROOT=$root RUSTPRESS_TEST_WP_ROOT=$root RUST_MIN_STACK=${RUST_MIN_STACK:-16777216}
exec cargo test --workspace "$@"
