# Runbooks And Recovery

OpenFoundry already stores several operational references directly in the repository.

## Existing Runbooks

| Path | Purpose |
| --- | --- |
| `infra/runbooks/disaster-recovery.md` | Ordered disaster recovery procedure for Compose and Kubernetes environments. |
| `infra/runbooks/upgrade-playbook.md` | Canary, promotion, and rollback guidance for upgrades. |

## Backup And Restore Scripts

| Script | Purpose |
| --- | --- |
| `infra/scripts/postgres_backup.sh` | Postgres backup helper. |
| `infra/scripts/postgres_restore.sh` | Postgres restore helper. |
| `infra/scripts/minio_backup.sh` | Object storage backup helper. |
| `infra/scripts/minio_restore.sh` | Object storage restore helper. |

## Operational Positioning

These files are important because the repo already treats operations as code:

- deployment profiles are versioned
- validation flows are automated
- recovery procedures live next to deployable assets

As the platform grows, this section is a good place to add service-specific SLOs, incident checklists, and recovery matrices.
