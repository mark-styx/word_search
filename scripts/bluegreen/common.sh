#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STATE_DIR="${ROOT_DIR}/.bluegreen"
STATE_FILE="${STATE_DIR}/state.env"
BACKUP_DIR="${STATE_DIR}/backups"
GENERATED_CADDYFILE="${STATE_DIR}/Caddyfile"

LIVE_HOSTNAME="${WORDSEARCH_LIVE_HOSTNAME:-wordsearch.home.arpa}"
PREVIEW_HOSTNAME="${WORDSEARCH_PREVIEW_HOSTNAME:-wordsearch-preview.home.arpa}"
LEGACY_LIVE_BACKEND_CONTAINER="${WORDSEARCH_LEGACY_LIVE_BACKEND_CONTAINER:-word-search-backend}"
PORT_AUTHORITY_BIN="${WORDSEARCH_PORT_AUTHORITY_BIN:-pa}"

mkdir -p "${STATE_DIR}" "${BACKUP_DIR}"

compose_runner() {
  if docker compose version >/dev/null 2>&1; then
    echo "docker compose"
    return 0
  fi

  if command -v docker-compose >/dev/null 2>&1; then
    echo "docker-compose"
    return 0
  fi

  echo "neither 'docker compose' nor 'docker-compose' is available" >&2
  return 1
}

port_authority_available() {
  command -v "${PORT_AUTHORITY_BIN}" >/dev/null 2>&1
}

port_authority_lookup() {
  local project="$1"
  local lookup_output port

  if ! port_authority_available; then
    return 1
  fi

  if ! lookup_output="$("${PORT_AUTHORITY_BIN}" lookup "${project}" 2>/dev/null)"; then
    return 1
  fi

  port="$(printf '%s\n' "${lookup_output}" | sed -n 's/.*port \([0-9][0-9]*\).*/\1/p' | head -n 1)"
  if [[ -z "${port}" ]]; then
    return 1
  fi

  echo "${port}"
}

stack_port_authority_project() {
  local stack="$1"
  local service="$2"

  case "${stack}:${service}" in
    blue:frontend) echo "word-search-blue-frontend" ;;
    blue:backend) echo "word-search-blue-backend" ;;
    green:frontend) echo "word-search-green-frontend" ;;
    green:backend) echo "word-search-green-backend" ;;
    *)
      echo "unknown stack/service combination: ${stack}/${service}" >&2
      return 1
      ;;
  esac
}

stack_default_frontend_port() {
  case "$1" in
    blue) echo 3089 ;;
    green) echo 4189 ;;
    *) echo "unknown stack: $1" >&2; return 1 ;;
  esac
}

stack_default_backend_port() {
  case "$1" in
    blue) echo 3099 ;;
    green) echo 4199 ;;
    *) echo "unknown stack: $1" >&2; return 1 ;;
  esac
}

stack_frontend_port() {
  local stack="$1"
  local env_name project port

  case "${stack}" in
    blue) env_name="WORDSEARCH_BLUE_FRONTEND_PORT" ;;
    green) env_name="WORDSEARCH_GREEN_FRONTEND_PORT" ;;
    *) echo "unknown stack: ${stack}" >&2; return 1 ;;
  esac

  if [[ -n "${!env_name:-}" ]]; then
    echo "${!env_name}"
    return 0
  fi

  project="$(stack_port_authority_project "${stack}" frontend)"
  if port="$(port_authority_lookup "${project}")"; then
    echo "${port}"
    return 0
  fi

  stack_default_frontend_port "${stack}"
}

stack_backend_port() {
  local stack="$1"
  local env_name project port

  case "${stack}" in
    blue) env_name="WORDSEARCH_BLUE_BACKEND_PORT" ;;
    green) env_name="WORDSEARCH_GREEN_BACKEND_PORT" ;;
    *) echo "unknown stack: ${stack}" >&2; return 1 ;;
  esac

  if [[ -n "${!env_name:-}" ]]; then
    echo "${!env_name}"
    return 0
  fi

  project="$(stack_port_authority_project "${stack}" backend)"
  if port="$(port_authority_lookup "${project}")"; then
    echo "${port}"
    return 0
  fi

  stack_default_backend_port "${stack}"
}

stack_project() {
  case "$1" in
    blue|green) echo "word-search-$1" ;;
    *) echo "unknown stack: $1" >&2; return 1 ;;
  esac
}

stack_backend_container() {
  case "$1" in
    blue|green) echo "word-search-backend-$1" ;;
    *) echo "unknown stack: $1" >&2; return 1 ;;
  esac
}

stack_frontend_container() {
  case "$1" in
    blue|green) echo "word-search-frontend-$1" ;;
    *) echo "unknown stack: $1" >&2; return 1 ;;
  esac
}

stack_volume() {
  case "$1" in
    blue|green) echo "word-search-$1-backend-data" ;;
    *) echo "unknown stack: $1" >&2; return 1 ;;
  esac
}

stack_public_url() {
  local stack="$1"
  local frontend_port
  frontend_port="$(stack_frontend_port "${stack}")"
  echo "${WORDSEARCH_HOST_BASE_URL:-http://127.0.0.1}:${frontend_port}"
}

container_exists() {
  docker inspect "$1" >/dev/null 2>&1
}

default_state() {
  cat <<EOF
ACTIVE_STACK=blue
PREVIEW_STACK=green
EOF
}

ensure_state() {
  if [[ ! -f "${STATE_FILE}" ]]; then
    default_state > "${STATE_FILE}"
  fi

  # shellcheck disable=SC1090
  source "${STATE_FILE}"

  ACTIVE_STACK="${ACTIVE_STACK:-blue}"
  PREVIEW_STACK="${PREVIEW_STACK:-green}"
}

save_state() {
  cat > "${STATE_FILE}" <<EOF
ACTIVE_STACK=${ACTIVE_STACK}
PREVIEW_STACK=${PREVIEW_STACK}
EOF
}

inactive_stack() {
  case "${ACTIVE_STACK}" in
    blue) echo green ;;
    green) echo blue ;;
    *) echo "invalid ACTIVE_STACK: ${ACTIVE_STACK}" >&2; return 1 ;;
  esac
}

compose_stack() {
  local stack="$1"
  shift
  local compose_cmd
  compose_cmd="$(compose_runner)"

  STACK_BACKEND_CONTAINER="$(stack_backend_container "${stack}")" \
  STACK_FRONTEND_CONTAINER="$(stack_frontend_container "${stack}")" \
  STACK_VOLUME_NAME="$(stack_volume "${stack}")" \
  FRONTEND_PORT="$(stack_frontend_port "${stack}")" \
  BACKEND_PORT="$(stack_backend_port "${stack}")" \
  PUBLIC_FRONTEND_URL="$(stack_public_url "${stack}")" \
  ${compose_cmd} \
  -p "$(stack_project "${stack}")" \
  -f "${ROOT_DIR}/docker-compose.bluegreen.yml" \
  "$@"
}

active_frontend_port() {
  stack_frontend_port "${ACTIVE_STACK}"
}

preview_frontend_port() {
  stack_frontend_port "${PREVIEW_STACK}"
}

render_caddyfile() {
  cat > "${GENERATED_CADDYFILE}" <<EOF
{
	auto_https off
}

http://${LIVE_HOSTNAME} {
	reverse_proxy 127.0.0.1:$(active_frontend_port)
}

http://${PREVIEW_HOSTNAME} {
	reverse_proxy 127.0.0.1:$(preview_frontend_port)
}
EOF
}

default_caddyfile_target() {
  if [[ -d /opt/homebrew/etc ]]; then
    echo /opt/homebrew/etc/Caddyfile
    return 0
  fi

  if [[ -d /usr/local/etc ]]; then
    echo /usr/local/etc/Caddyfile
    return 0
  fi

  return 1
}

publish_caddyfile_if_configured() {
  local caddyfile_target="${WORDSEARCH_CADDYFILE_TARGET:-}"
  local caddy_admin="${WORDSEARCH_CADDY_ADMIN-localhost:2019}"

  if [[ -z "${caddyfile_target}" ]] && command -v caddy >/dev/null 2>&1; then
    caddyfile_target="$(default_caddyfile_target || true)"
  fi

  if [[ -n "${caddyfile_target}" ]]; then
    cp "${GENERATED_CADDYFILE}" "${caddyfile_target}"
  fi

  if command -v caddy >/dev/null 2>&1; then
    caddy validate --config "${GENERATED_CADDYFILE}" >/dev/null
    if [[ -n "${caddy_admin}" ]]; then
      caddy reload --address "${caddy_admin}" --config "${GENERATED_CADDYFILE}" >/dev/null
    fi
  fi
}

backup_active_db() {
  ensure_state
  local timestamp backup_path active_container
  timestamp="$(date +%Y%m%d-%H%M%S)"
  backup_path="${BACKUP_DIR}/wordsearch-${ACTIVE_STACK}-${timestamp}.db"
  active_container="$(stack_backend_container "${ACTIVE_STACK}")"

  if ! container_exists "${active_container}"; then
    active_container="${LEGACY_LIVE_BACKEND_CONTAINER}"
  fi

  docker cp "${active_container}:/app/data/wordsearch.db" "${backup_path}"
  # Collect the WAL/SHM siblings when present: the db file alone can be a few
  # transactions behind on a busy live server. The triple restores cleanly.
  docker cp "${active_container}:/app/data/wordsearch.db-wal" "${backup_path}-wal" 2>/dev/null || true
  docker cp "${active_container}:/app/data/wordsearch.db-shm" "${backup_path}-shm" 2>/dev/null || true
  echo "${backup_path}"
}

# Copy the live SQLite database from one stack's volume into another's.
# Deploy-time best effort (same snapshot semantics as backup_active_db):
# grabs db + wal + shm, keeps a .pre-sync safety copy in the target, then
# restarts the target backend so it opens the fresh files. The target stack
# is started first if it is not running; a subshell keeps the EXIT trap local.
sync_db_between_stacks() {
  local from_stack="$1" to_stack="$2"
  if [[ "${from_stack}" == "${to_stack}" ]]; then
    echo "refusing to sync stack ${from_stack} into itself" >&2
    return 1
  fi

  local from_container
  from_container="$(stack_backend_container "${from_stack}")"
  if ! container_exists "${from_container}"; then
    from_container="${LEGACY_LIVE_BACKEND_CONTAINER}"
  fi
  if ! container_exists "${from_container}"; then
    echo "source backend container not running: ${from_container}" >&2
    return 1
  fi

  compose_stack "${to_stack}" up -d backend >/dev/null

  (
    set -euo pipefail
    to_container="$(stack_backend_container "${to_stack}")"
    if ! container_exists "${to_container}"; then
      echo "target backend container failed to start: ${to_container}" >&2
      exit 1
    fi
    tmp_dir="$(mktemp -d /tmp/wordsearch-stack-sync.XXXXXX)"
    trap 'rm -rf "${tmp_dir}"' EXIT

    docker cp "${from_container}:/app/data/wordsearch.db" "${tmp_dir}/wordsearch.db"
    docker cp "${from_container}:/app/data/wordsearch.db-wal" "${tmp_dir}/wordsearch.db-wal" 2>/dev/null || true
    docker cp "${from_container}:/app/data/wordsearch.db-shm" "${tmp_dir}/wordsearch.db-shm" 2>/dev/null || true

    if docker exec "${to_container}" test -f /app/data/wordsearch.db; then
      docker exec "${to_container}" sh -lc "cp /app/data/wordsearch.db /app/data/wordsearch.pre-sync.$(date +%Y%m%d-%H%M%S).db" || true
    fi
    docker exec "${to_container}" sh -lc "rm -f /app/data/wordsearch.db /app/data/wordsearch.db-wal /app/data/wordsearch.db-shm"
    docker cp "${tmp_dir}/wordsearch.db" "${to_container}:/app/data/wordsearch.db"
    if [[ -f "${tmp_dir}/wordsearch.db-wal" ]]; then
      docker cp "${tmp_dir}/wordsearch.db-wal" "${to_container}:/app/data/wordsearch.db-wal"
    fi
    if [[ -f "${tmp_dir}/wordsearch.db-shm" ]]; then
      docker cp "${tmp_dir}/wordsearch.db-shm" "${to_container}:/app/data/wordsearch.db-shm"
    fi
    compose_stack "${to_stack}" restart backend >/dev/null
  )

  echo "synced_db_from=${from_container}"
  echo "synced_db_to=$(stack_backend_container "${to_stack}")"
}
