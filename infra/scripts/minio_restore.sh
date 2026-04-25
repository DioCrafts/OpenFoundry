#!/usr/bin/env sh
set -eu

: "${OPENFOUNDRY_MC_ALIAS:=local}"
: "${OPENFOUNDRY_BACKUP_DIR:=./backups/minio}"

for bucket in openfoundry openfoundry-datasets openfoundry-notebooks openfoundry-artifacts; do
  if [ ! -d "${OPENFOUNDRY_BACKUP_DIR}/${bucket}" ]; then
    echo "Skipping ${bucket}: no backup directory found" >&2
    continue
  fi
  echo "Restoring ${bucket}"
  mc mirror --overwrite "${OPENFOUNDRY_BACKUP_DIR}/${bucket}" "${OPENFOUNDRY_MC_ALIAS}/${bucket}"
done

echo "MinIO restore completed"
