#!/usr/bin/env sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
test_root=$(mktemp -d "${TMPDIR:-/tmp}/kanterm-installer-test.XXXXXX")
trap 'rm -rf "$test_root"' EXIT

case "$(uname -s):$(uname -m)" in
  Linux:x86_64|Linux:amd64) platform=linux-x86_64 ;;
  Darwin:arm64|Darwin:aarch64) platform=macos-arm64 ;;
  *)
    printf '%s\n' 'installer test skipped on an unsupported release platform'
    exit 0
    ;;
esac

if command -v sha256sum >/dev/null 2>&1; then
  checksum() { sha256sum "$1" | awk '{print $1}'; }
else
  checksum() { shasum -a 256 "$1" | awk '{print $1}'; }
fi

fixture_root=$test_root/fixtures
version=v9.9.9

make_release() {
  product=$1
  shift
  release_dir=$fixture_root/$product/releases/download/$version
  payload_dir=$test_root/payload/$product-$platform
  archive=$release_dir/$product-$platform.tar.gz
  mkdir -p "$release_dir" "$payload_dir"

  for binary_name in "$@"; do
    printf '#!/usr/bin/env sh\nprintf "%%s\\n" "%s fixture"\n' "$binary_name" \
      > "$payload_dir/$binary_name"
    chmod 755 "$payload_dir/$binary_name"
  done

  tar -C "$test_root/payload" -czf "$archive" "$product-$platform"
  printf '%s  %s\n' "$(checksum "$archive")" "$(basename "$archive")" \
    > "$release_dir/SHA256SUMS"
}

make_release kanterm kanterm kanterm-mcp
make_release kanpty kanpty kanptyd

fake_bin=$test_root/fake-bin
mkdir -p "$fake_bin"
cat > "$fake_bin/curl" <<'FAKE_CURL'
#!/usr/bin/env sh
set -eu
output=
url=
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output)
      shift
      output=$1
      ;;
    https://fixtures.invalid/*)
      url=$1
      ;;
  esac
  shift
done
[ -n "$output" ]
[ -n "$url" ]
relative=${url#https://fixtures.invalid/}
cp "$FIXTURE_ROOT/$relative" "$output"
FAKE_CURL
chmod 755 "$fake_bin/curl"

install_dir=$test_root/bin
PATH=$fake_bin:$PATH \
FIXTURE_ROOT=$fixture_root \
KANTERM_RELEASE_URL=https://fixtures.invalid/kanterm/releases \
KANPTY_RELEASE_URL=https://fixtures.invalid/kanpty/releases \
  sh "$repo_root/install.sh" \
    --install-dir "$install_dir" \
    --kanterm-version "$version" \
    --kanpty-version 9.9.9

for binary_name in kanterm kanterm-mcp kanpty kanptyd; do
  [ -x "$install_dir/$binary_name" ]
  [ "$("$install_dir/$binary_name")" = "$binary_name fixture" ]
done

printf '%s\n' 'existing install' > "$install_dir/kanterm"
archive=$fixture_root/kanterm/releases/download/$version/kanterm-$platform.tar.gz
printf '%s\n' 'corrupt archive' > "$archive"
if PATH=$fake_bin:$PATH \
  FIXTURE_ROOT=$fixture_root \
  KANTERM_RELEASE_URL=https://fixtures.invalid/kanterm/releases \
  KANPTY_RELEASE_URL=https://fixtures.invalid/kanpty/releases \
    sh "$repo_root/install.sh" \
      --install-dir "$install_dir" \
      --kanterm-version "$version" \
      --kanpty-version "$version" >/dev/null 2>&1
then
  printf '%s\n' 'checksum failure unexpectedly succeeded' >&2
  exit 1
fi
[ "$(cat "$install_dir/kanterm")" = 'existing install' ]

dry_run_output=$(sh "$repo_root/install.sh" \
  --install-dir "$test_root/dry-run" \
  --kanterm-version 1.2.3 \
  --kanpty-version v2.3.4 \
  --dry-run)
printf '%s\n' "$dry_run_output" | grep 'Kanterm version: v1.2.3' >/dev/null
printf '%s\n' "$dry_run_output" | grep 'Kanpty version:  v2.3.4' >/dev/null
[ ! -e "$test_root/dry-run" ]

printf '%s\n' 'installer tests passed'
