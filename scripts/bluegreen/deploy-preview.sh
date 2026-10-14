#!/usr/bin/env bash

set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/common.sh"

ensure_state

target_stack="${1:-${PREVIEW_STACK}}"

if [[ "${target_stack}" != "blue" && "${target_stack}" != "green" ]]; then
  echo "usage: $0 [blue|green]" >&2
  exit 1
fi

if [[ "${target_stack}" == "${ACTIVE_STACK}" ]]; then
  echo "refusing to deploy preview onto the active live stack (${ACTIVE_STACK})" >&2
  exit 1
fi

PREVIEW_STACK="${target_stack}"
save_state

compose_stack "${target_stack}" build backend frontend
compose_stack "${target_stack}" up -d --force-recreate backend frontend

render_caddyfile
publish_caddyfile_if_configured

cat <<EOF
deployed_stack=${target_stack}
preview_url=http://127.0.0.1:$(stack_frontend_port "${target_stack}")
preview_hostname=${PREVIEW_HOSTNAME}
EOF
