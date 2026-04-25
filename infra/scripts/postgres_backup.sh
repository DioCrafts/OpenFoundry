#!/usr/bin/env sh
set -eu

: "${OPENFOUNDRY_PGHOST:=127.0.0.1}"
: "${OPENFOUNDRY_PGPORT:=5432}"
: "${OPENFOUNDRY_PGUSER:=openfoundry}"
: "${OPENFOUNDRY_PGDATABASE:=openfoundry}"
: "${OPENFOUNDRY_BACKUP_DIR:=./backups}"

timestamp="$(date -u +"%Y%m%dT%H%M%SZ")"
mkdir -p "${OPENFOUNDRY_BACKUP_DIR}"

output="${OPENFOUNDRY_BACKUP_DIR}/openfoundry-postgres-${timestamp}.dump"

echo "Creating PostgreSQL backup at ${output}"
pg_dump \
  --host="${OPENFOUNDRY_PGHOST}" \
  --port="${OPENFOUNDRY_PGPORT}" \
  --username="${OPENFOUNDRY_PGUSER}" \
  --format=custom \
  --file="${output}" \
  "${OPENFOUNDRY_PGDATABASE}"

echo "Backup completed: ${output}"
