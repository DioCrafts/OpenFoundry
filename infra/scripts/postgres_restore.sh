#!/usr/bin/env sh
set -eu

: "${OPENFOUNDRY_PGHOST:=127.0.0.1}"
: "${OPENFOUNDRY_PGPORT:=5432}"
: "${OPENFOUNDRY_PGUSER:=openfoundry}"
: "${OPENFOUNDRY_PGDATABASE:=openfoundry}"

if [ "${1:-}" = "" ]; then
  echo "usage: $0 <backup.dump>" >&2
  exit 1
fi

input="$1"
echo "Restoring PostgreSQL backup from ${input}"
pg_restore \
  --clean \
  --if-exists \
  --no-owner \
  --host="${OPENFOUNDRY_PGHOST}" \
  --port="${OPENFOUNDRY_PGPORT}" \
  --username="${OPENFOUNDRY_PGUSER}" \
  --dbname="${OPENFOUNDRY_PGDATABASE}" \
  "${input}"

echo "Restore completed"
