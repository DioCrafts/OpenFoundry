#!/usr/bin/env sh
set -eu

: "${OPENFOUNDRY_MC_ALIAS:=local}"
: "${OPENFOUNDRY_BACKUP_DIR:=./backups/minio}"

mkdir -p "${OPENFOUNDRY_BACKUP_DIR}"

for bucket in openfoundry openfoundry-datasets openfoundry-notebooks openfoundry-artifacts; do
  echo "Backing up ${bucket}"
  mc mirror "${OPENFOUNDRY_MC_ALIAS}/${bucket}" "${OPENFOUNDRY_BACKUP_DIR}/${bucket}"
done

echo "MinIO backup completed in ${OPENFOUNDRY_BACKUP_DIR}"
