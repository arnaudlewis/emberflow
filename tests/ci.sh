#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

assert_file() {
  local path="$1"
  [ -f "$path" ] || { echo "FAIL: expected file: $path" >&2; exit 1; }
}

assert_contains() {
  local needle="$1"
  local path="$2"
  grep -qF -- "$needle" "$path" || { echo "FAIL: expected '$needle' in $path" >&2; exit 1; }
}

echo "==> cargo test (bins + integration)"
cargo test --manifest-path "$ROOT_DIR/Cargo.toml" --lib --bins --tests

echo "==> cargo test (doctests)"
cargo test --manifest-path "$ROOT_DIR/Cargo.toml" --doc

echo "==> layout checks"
bash "$ROOT_DIR/tests/layout.sh"

echo "==> adapter checks"
bash "$ROOT_DIR/tests/adapters.sh"

echo "==> minter ci"
(
  cd "$ROOT_DIR"
  minter ci
)

echo "==> release packaging smoke test"
HOST_TARGET="$(rustc -vV | awk '/^host: / {print $2}')"
(
  cd "$ROOT_DIR"
  CARGO_TARGET_DIR="$ROOT_DIR/target" cargo build --release --target "$HOST_TARGET" --bin emberflow --bin emberflow-mcp
)

STAGING_DIR="$(mktemp -d)"
SMOKE_ARCHIVE="${STAGING_DIR}.tar.gz"
cp "$ROOT_DIR/target/$HOST_TARGET/release/emberflow" "$STAGING_DIR/"
cp "$ROOT_DIR/target/$HOST_TARGET/release/emberflow-mcp" "$STAGING_DIR/"
cp "$ROOT_DIR/README.md" "$STAGING_DIR/README.md"
if [[ -f "$ROOT_DIR/LICENSE" ]]; then
  cp "$ROOT_DIR/LICENSE" "$STAGING_DIR/LICENSE"
fi
tar -czf "$SMOKE_ARCHIVE" -C "$STAGING_DIR" .
assert_file "$SMOKE_ARCHIVE"
rm -rf "$STAGING_DIR" "$SMOKE_ARCHIVE"

echo "==> workflow contract"
assert_file "$ROOT_DIR/.github/workflows/emberflow.yml"
assert_file "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_file "$ROOT_DIR/cliff.toml"
assert_contains "cargo fmt --all --check" "$ROOT_DIR/.github/workflows/emberflow.yml"
assert_contains "cargo clippy --all-targets -- -D warnings" "$ROOT_DIR/.github/workflows/emberflow.yml"
assert_contains "bash tests/ci.sh" "$ROOT_DIR/.github/workflows/emberflow.yml"
assert_contains "cargo package --allow-dirty" "$ROOT_DIR/.github/workflows/emberflow.yml"
assert_contains "workflow_dispatch:" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "default: false" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "run-name: \"\${{ inputs.execute && '[Prod]' || '[DryRun]' }} EmberFlow Release\"" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Prepare Release" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "CI Gate" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "runs-on: macos-14" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Check formatting" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Run clippy" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Bump & Tag" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Build (" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Generate changelog preview" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Generate release changelog" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "orhun/git-cliff-action@v4" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "config: cliff.toml" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "CARGO_TARGET_DIR: target" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Publish Release" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Update Homebrew Tap" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "TAP_REPO: arnaudlewis/homebrew-tap" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "brew install arnaudlewis/tap/emberflow" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "aarch64-apple-darwin" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "x86_64-apple-darwin" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "x86_64-unknown-linux-gnu" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "aarch64-unknown-linux-gnu" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "cargo build --release --target" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "cross build --release --target" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "tar -czf" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "sha256sum * > SHA256SUMS.txt" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "cp \"target/\${PLATFORM}/release/emberflow\" \"\$STAGING/\"" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "cp \"target/\${PLATFORM}/release/emberflow-mcp\" \"\$STAGING/\"" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "cp README.md \"\$STAGING/README.md\"" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "cp LICENSE \"\$STAGING/LICENSE\"" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "bin.install \"emberflow\"" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "bin.install \"emberflow-mcp\"" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "softprops/action-gh-release@v2" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "Dry run (CI plus build/package validation only; no tag, publish, or tap update)" "$ROOT_DIR/.github/workflows/emberflow-release.yml"
assert_contains "All notable changes to EmberFlow will be documented in this file." "$ROOT_DIR/cliff.toml"
assert_contains "tag_pattern = \"emberflow-v[0-9].*\"" "$ROOT_DIR/cliff.toml"
