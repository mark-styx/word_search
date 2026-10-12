#!/usr/bin/env bash

# Manually refresh the preview stack's database from the live stack.
# (promote.sh does this automatically now; this remains useful to rehearse a
# deploy against real data, or to reset preview after experiments.)

set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/common.sh"

ensure_state

preview_stack="${1:-${PREVIEW_STACK}}"

if [[ "${preview_stack}" == "${ACTIVE_STACK}" ]]; then
  echo "refusing to sync the active live stack into itself (${ACTIVE_STACK})" >&2
  exit 1
fi

sync_db_between_stacks "${ACTIVE_STACK}" "${preview_stack}"
