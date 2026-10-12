#!/usr/bin/env bash

# Roll back to the previously-active stack. Anything written to the database
# while the failed deployment was live is carried back with it (each stack
# has its own volume; without the sync a rollback silently abandons those
# rows). Safety backup taken first, same as promote.

set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/common.sh"

ensure_state

rollback_target="$(inactive_stack)"

backup_path="$(backup_active_db)"
sync_output="$(sync_db_between_stacks "${ACTIVE_STACK}" "${rollback_target}")"

ACTIVE_STACK="${rollback_target}"
PREVIEW_STACK="$(inactive_stack)"
save_state

render_caddyfile
publish_caddyfile_if_configured

cat <<EOF
rolled_back_to=${ACTIVE_STACK}
preview_stack=${PREVIEW_STACK}
backup=${backup_path}
${sync_output}
EOF
