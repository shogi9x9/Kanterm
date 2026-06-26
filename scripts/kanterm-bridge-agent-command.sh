#!/usr/bin/env sh
set -eu

usage() {
  cat >&2 <<'USAGE'
usage: kanterm-bridge-agent-command.sh --repo PATH -- COMMAND [ARG...]

Runs COMMAND in the target repo and passes a formatted Kanterm handoff prompt on
stdin. Use this as a generic bridge for local agent CLIs or custom supervisors.
USAGE
}

repo=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo)
      shift
      [ "$#" -gt 0 ] || { echo "--repo requires a value" >&2; exit 2; }
      repo=$1
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    *)
      echo "unknown argument before --: $1" >&2
      usage
      exit 2
      ;;
  esac
  shift
done

[ -n "$repo" ] || { echo "--repo is required" >&2; exit 2; }
[ -d "$repo" ] || { echo "repo does not exist: $repo" >&2; exit 2; }
[ "$#" -gt 0 ] || { echo "COMMAND is required after --" >&2; exit 2; }

body=$(cat)

cd "$repo"
{
  printf 'Kanterm handoff received.\n\n'
  printf 'handoff_id: %s\n' "${KANTERM_HANDOFF_ID:-}"
  printf 'from_agent: %s\n' "${KANTERM_HANDOFF_FROM_AGENT:-}"
  printf 'to_agent: %s\n' "${KANTERM_HANDOFF_TO_AGENT:-}"
  printf 'subject: %s\n' "${KANTERM_HANDOFF_SUBJECT:-}"
  printf 'board_id: %s\n' "${KANTERM_HANDOFF_BOARD_ID:-}"
  printf 'card_key: %s\n' "${KANTERM_HANDOFF_CARD_KEY:-}"
  printf 'lease_expires_at: %s\n\n' "${KANTERM_HANDOFF_LEASE_EXPIRES_AT:-}"
  printf 'Task:\n%s\n' "$body"
} | "$@"
