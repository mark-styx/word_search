#!/usr/bin/env bash

# Promote the preview stack to live. The live database is carried across
# automatically: each stack keeps its own Docker volume, so without the sync
# a promote would serve the target stack's stale/empty data ("players lost
# their accounts"). A safety backup of the active DB is taken first.

set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/common.sh"

ensure_state

target_stack="${1:-${PREVIEW_STACK}}"

if [[ "${target_stack}" != "blue" && "${target_stack}" != "green" ]]; then
  echo "usage: $0 [blue|green]" >&2
  exit 1
fi
if [[ "${target_stack}" == "${ACTIVE_STACK}" ]]; then
  echo "refusing to promote the already-active stack (${ACTIVE_STACK})" >&2
  exit 1
fi

backup_path="$(backup_active_db)"
sync_output="$(sync_db_between_stacks "${ACTIVE_STACK}" "${target_stack}")"

ACTIVE_STACK="${target_stack}"
PREVIEW_STACK="$(inactive_stack)"
save_state

render_caddyfile
publish_caddyfile_if_configured

cat <<EOF
promoted_stack=${ACTIVE_STACK}
preview_stack=${PREVIEW_STACK}
backup=${backup_path}
${sync_output}
live_hostname=${LIVE_HOSTNAME}
preview_hostname=${PREVIEW_HOSTNAME}
EOF
