#!/usr/bin/env bash

# Nightly database backup for the word_search box: timestamped SQLite triple
# (db + wal + shm) from the active stack into .bluegreen/backups/, keeping
# the newest 30 sets. Install on the host with:
#
#   crontab -e
#   17 3 * * * cd /path/to/word_search && ./scripts/backup-db.sh >> .bluegreen/backups/backup.log 2>&1
#
# Backups live on the same disk as the containers, so the durable upgrade is
# Litestream streaming to S3/R2 (same Cloudflare account) — see DEPLOYMENT.md.

set -euo pipefail

source "$(cd "$(dirname "$0")" && pwd)/bluegreen/common.sh"

ensure_state

backup_path="$(backup_active_db)"

# Retention: keep the 30 newest backup sets (db plus -wal/-shm siblings).
# (while-read instead of mapfile: macOS still ships bash 3.2.)
kept=0
while IFS= read -r old; do
  kept=$((kept + 1))
  if (( kept > 30 )); then
    rm -f "${old}" "${old}-wal" "${old}-shm"
  fi
done < <(ls -1t "${BACKUP_DIR}"/wordsearch-*.db 2>/dev/null || true)

echo "$(date -u +%FT%TZ) backup=${backup_path} total=$(ls -1 "${BACKUP_DIR}"/wordsearch-*.db 2>/dev/null | wc -l | tr -d ' ')"
