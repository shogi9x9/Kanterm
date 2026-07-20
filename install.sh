#!/usr/bin/env sh
set -eu

DEFAULT_KANPTY_VERSION=v0.2.0
KANTERM_RELEASE_URL=${KANTERM_RELEASE_URL:-https://github.com/shogi9x9/Kanterm/releases}
KANPTY_RELEASE_URL=${KANPTY_RELEASE_URL:-https://github.com/shogi9x9/Kanpty/releases}
kanterm_version=${KANTERM_VERSION:-latest}
kanpty_version=${KANPTY_VERSION:-$DEFAULT_KANPTY_VERSION}
dry_run=false

if [ -n "${INSTALL_DIR:-}" ]; then
  install_dir=$INSTALL_DIR
elif [ -n "${HOME:-}" ]; then
  install_dir=$HOME/.local/bin
else
  printf '%s\n' 'HOME is unset; pass --install-dir PATH or set INSTALL_DIR' >&2
  exit 2
fi

usage() {
  cat <<'USAGE'
Install Kanterm, kanterm-mcp, Kanpty, and kanptyd from verified GitHub releases.

usage: install.sh [options]

options:
  --install-dir PATH       destination directory (default: $HOME/.local/bin)
  --kanterm-version VALUE  Kanterm tag, without or without leading v (default: latest)
  --kanpty-version VALUE   Kanpty tag, without or without leading v (default: v0.2.0)
  --dry-run                print the resolved installation plan without downloading
  -h, --help               show this help

The same values can be supplied with INSTALL_DIR, KANTERM_VERSION, and
KANPTY_VERSION. Kanpty is pinned by default because Kanterm requires its
protocol v2 stdin-paste and stable-alias contract.
USAGE
}

die() {
  printf 'install: %s\n' "$*" >&2
  exit 1
}

require_value() {
  [ "$#" -ge 2 ] || {
    printf 'install: %s requires a value\n' "$1" >&2
    exit 2
  }
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --install-dir)
      require_value "$@"
      shift
      install_dir=$1
      ;;
    --kanterm-version)
      require_value "$@"
      shift
      kanterm_version=$1
      ;;
    --kanpty-version)
      require_value "$@"
      shift
      kanpty_version=$1
      ;;
    --dry-run)
      dry_run=true
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf 'install: unknown argument: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

normalize_version() {
  value=$1
  case "$value" in
    latest)
      printf '%s\n' latest
      return
      ;;
    '')
      die 'version must not be empty'
      ;;
    *[!A-Za-z0-9._-]*)
      die "invalid version: $value"
      ;;
    v*)
      printf '%s\n' "$value"
      ;;
    *)
      printf 'v%s\n' "$value"
      ;;
  esac
}

kanterm_version=$(normalize_version "$kanterm_version")
kanpty_version=$(normalize_version "$kanpty_version")

case "$install_dir" in
  /*) ;;
  *) die "install directory must be absolute: $install_dir" ;;
esac

os=$(uname -s)
arch=$(uname -m)
case "$os:$arch" in
  Linux:x86_64|Linux:amd64)
    platform=linux-x86_64
    ;;
  Darwin:arm64|Darwin:aarch64)
    platform=macos-arm64
    ;;
  *)
    die "unsupported platform: $os $arch (supported: Linux x86_64, macOS arm64)"
    ;;
esac

printf 'Kanterm version: %s\n' "$kanterm_version"
printf 'Kanpty version:  %s\n' "$kanpty_version"
printf 'Platform:        %s\n' "$platform"
printf 'Install dir:     %s\n' "$install_dir"

if [ "$dry_run" = true ]; then
  exit 0
fi

for command_name in curl tar awk mktemp install mv mkdir rm uname; do
  command -v "$command_name" >/dev/null 2>&1 || die "required command not found: $command_name"
done

if command -v sha256sum >/dev/null 2>&1; then
  checksum_file() {
    sha256sum "$1" | awk '{print $1}'
  }
elif command -v shasum >/dev/null 2>&1; then
  checksum_file() {
    shasum -a 256 "$1" | awk '{print $1}'
  }
else
  die 'sha256sum or shasum is required'
fi

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/kanterm-install.XXXXXX")
staged_kanterm=
staged_kanterm_mcp=
staged_kanpty=
staged_kanptyd=

cleanup() {
  [ -z "$staged_kanterm" ] || rm -f "$staged_kanterm"
  [ -z "$staged_kanterm_mcp" ] || rm -f "$staged_kanterm_mcp"
  [ -z "$staged_kanpty" ] || rm -f "$staged_kanpty"
  [ -z "$staged_kanptyd" ] || rm -f "$staged_kanptyd"
  [ ! -d "$work_dir" ] || rm -rf "$work_dir"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

fetch() {
  fetch_url=$1
  fetch_destination=$2
  curl \
    --proto '=https' \
    --tlsv1.2 \
    --fail \
    --location \
    --silent \
    --show-error \
    --retry 3 \
    --output "$fetch_destination" \
    "$fetch_url"
}

download_release() {
  product=$1
  release_url=$2
  version=$3
  asset="$product-$platform.tar.gz"
  product_dir=$work_dir/$product
  extract_dir=$product_dir/extract

  if [ "$version" = latest ]; then
    download_url=$release_url/latest/download
  else
    download_url=$release_url/download/$version
  fi

  mkdir -p "$extract_dir"
  printf 'Downloading %s %s...\n' "$product" "$version"
  fetch "$download_url/SHA256SUMS" "$product_dir/SHA256SUMS"
  fetch "$download_url/$asset" "$product_dir/$asset"

  expected_checksum=$(awk -v wanted="$asset" '
    {
      name = $2
      sub(/^\*/, "", name)
      if (name == wanted) {
        print $1
        exit
      }
    }
  ' "$product_dir/SHA256SUMS")
  [ -n "$expected_checksum" ] || die "$asset is missing from $product SHA256SUMS"
  actual_checksum=$(checksum_file "$product_dir/$asset")
  [ "$actual_checksum" = "$expected_checksum" ] || die "checksum mismatch for $asset"

  archive_root=$product-$platform
  if ! tar -tzf "$product_dir/$asset" | awk -v root="$archive_root" '
    BEGIN { found = 0 }
    {
      if ($0 == root || index($0, root "/") == 1) {
        if ($0 ~ /(^|\/)\.\.(\/|$)/) exit 1
        found = 1
        next
      }
      exit 1
    }
    END { if (!found) exit 1 }
  '; then
    die "archive contains an unexpected path: $asset"
  fi

  tar -xzf "$product_dir/$asset" -C "$extract_dir"
}

download_release kanterm "$KANTERM_RELEASE_URL" "$kanterm_version"
download_release kanpty "$KANPTY_RELEASE_URL" "$kanpty_version"

kanterm_source=$work_dir/kanterm/extract/kanterm-$platform
kanpty_source=$work_dir/kanpty/extract/kanpty-$platform
for source_binary in \
  "$kanterm_source/kanterm" \
  "$kanterm_source/kanterm-mcp" \
  "$kanpty_source/kanpty" \
  "$kanpty_source/kanptyd"
do
  [ -f "$source_binary" ] || die "archive is missing binary: $source_binary"
done

mkdir -p "$install_dir"
staged_kanterm=$(mktemp "$install_dir/.kanterm.install.XXXXXX")
staged_kanterm_mcp=$(mktemp "$install_dir/.kanterm-mcp.install.XXXXXX")
staged_kanpty=$(mktemp "$install_dir/.kanpty.install.XXXXXX")
staged_kanptyd=$(mktemp "$install_dir/.kanptyd.install.XXXXXX")

install -m 755 "$kanterm_source/kanterm" "$staged_kanterm"
install -m 755 "$kanterm_source/kanterm-mcp" "$staged_kanterm_mcp"
install -m 755 "$kanpty_source/kanpty" "$staged_kanpty"
install -m 755 "$kanpty_source/kanptyd" "$staged_kanptyd"

mv -f "$staged_kanterm" "$install_dir/kanterm"
staged_kanterm=
mv -f "$staged_kanterm_mcp" "$install_dir/kanterm-mcp"
staged_kanterm_mcp=
mv -f "$staged_kanpty" "$install_dir/kanpty"
staged_kanpty=
mv -f "$staged_kanptyd" "$install_dir/kanptyd"
staged_kanptyd=

printf 'Installed:\n'
printf '  %s/kanterm\n' "$install_dir"
printf '  %s/kanterm-mcp\n' "$install_dir"
printf '  %s/kanpty\n' "$install_dir"
printf '  %s/kanptyd\n' "$install_dir"

case ":${PATH:-}:" in
  *":$install_dir:"*) ;;
  *) printf 'Add %s to PATH before running the installed commands.\n' "$install_dir" >&2 ;;
esac
