#!/usr/bin/env bash

set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/common.sh"

ensure_state

cat <<EOF
active_stack=${ACTIVE_STACK}
preview_stack=${PREVIEW_STACK}
live_url=http://127.0.0.1:$(active_frontend_port)
preview_url=http://127.0.0.1:$(preview_frontend_port)
live_hostname=${LIVE_HOSTNAME}
preview_hostname=${PREVIEW_HOSTNAME}
blue_project=$(stack_project blue)
green_project=$(stack_project green)
EOF
