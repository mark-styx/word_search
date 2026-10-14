#!/usr/bin/env bash

set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/common.sh"

ensure_state
render_caddyfile
publish_caddyfile_if_configured

cat <<EOF
published_caddyfile=${GENERATED_CADDYFILE}
live_hostname=${LIVE_HOSTNAME}
preview_hostname=${PREVIEW_HOSTNAME}
live_upstream=127.0.0.1:$(active_frontend_port)
preview_upstream=127.0.0.1:$(preview_frontend_port)
EOF
