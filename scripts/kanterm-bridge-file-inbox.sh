#!/usr/bin/env sh
set -eu

usage() {
  cat >&2 <<'USAGE'
usage: kanterm-bridge-file-inbox.sh --repo PATH [--inbox-dir DIR]

Reads a Kanterm handoff body from stdin and writes a Markdown inbox file in the
target repo. This bridge is runtime-neutral: a human, hook, or another watcher
can pick up the file later.
USAGE
}

repo=
inbox_dir=".kanterm/inbox"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo)
      shift
      [ "$#" -gt 0 ] || { echo "--repo requires a value" >&2; exit 2; }
      repo=$1
      ;;
    --inbox-dir)
      shift
      [ "$#" -gt 0 ] || { echo "--inbox-dir requires a value" >&2; exit 2; }
      inbox_dir=$1
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
  shift
done

[ -n "$repo" ] || { echo "--repo is required" >&2; exit 2; }
[ -d "$repo" ] || { echo "repo does not exist: $repo" >&2; exit 2; }

handoff_id=${KANTERM_HANDOFF_ID:-unknown}
subject=${KANTERM_HANDOFF_SUBJECT:-Handoff}
safe_id=$(printf '%s' "$handoff_id" | tr -c 'A-Za-z0-9_.-' '_')
dest_dir="$repo/$inbox_dir"
dest="$dest_dir/$safe_id.md"

mkdir -p "$dest_dir"
tmp="$dest.tmp.$$"
{
  printf '# %s\n\n' "$subject"
  printf -- '- handoff_id: %s\n' "$handoff_id"
  printf -- '- from_agent: %s\n' "${KANTERM_HANDOFF_FROM_AGENT:-}"
  printf -- '- to_agent: %s\n' "${KANTERM_HANDOFF_TO_AGENT:-}"
  printf -- '- board_id: %s\n' "${KANTERM_HANDOFF_BOARD_ID:-}"
  printf -- '- card_key: %s\n' "${KANTERM_HANDOFF_CARD_KEY:-}"
  printf -- '- lease_expires_at: %s\n\n' "${KANTERM_HANDOFF_LEASE_EXPIRES_AT:-}"
  printf '## Body\n\n'
  cat
  printf '\n'
} > "$tmp"
mv "$tmp" "$dest"
printf 'wrote %s\n' "$dest" >&2
