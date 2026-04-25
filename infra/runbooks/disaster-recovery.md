# Disaster Recovery Runbook

Fecha: 25 de abril de 2026

## Objetivo

Recuperar OpenFoundry ante pérdida parcial o total del plano de control, minimizando RTO y evitando restores inconsistentes entre PostgreSQL y object storage.

## Dependencias

- Snapshots o dumps recientes de PostgreSQL
- Backup del bucket principal de artefactos/datasets
- Credenciales para el cluster o el host de Compose
- Manifiestos Helm/Terraform del entorno afectado

## Orden de recuperación

1. Restaurar red, DNS, registry y secretos base
2. Restaurar PostgreSQL
3. Restaurar object storage
4. Levantar servicios stateless
5. Verificar migraciones
6. Reanudar schedulers, sync engines y reconciliadores
7. Ejecutar smoke checks funcionales

## Procedimiento Compose

1. Parar schedulers y reconciliadores para evitar escritura nueva
2. Restaurar PostgreSQL con `infra/scripts/postgres_restore.sh`
3. Restaurar buckets con `infra/scripts/minio_restore.sh`
4. Levantar `docker compose` con los mismos perfiles usados antes del incidente
5. Verificar salud de:
   - `gateway`
   - `auth-service`
   - `dataset-service`
   - `ontology-service`
   - `marketplace-service`

## Procedimiento Kubernetes

1. Escalar a `0` workloads con mutación o background jobs
2. Restaurar volúmenes o snapshots administrados
3. Reaplicar chart base y overlays de entorno
4. Rehabilitar cronjobs, reconcilers y autoscaling
5. Reejecutar checks de smoke y rutas críticas

## Smoke checks obligatorios

- Login y emisión de token
- Listado de datasets
- Listado de ontology object types
- Preview de pipeline
- Consulta de fleets DevOps
- Chat o AI provider health si el entorno lo usa

## Criterio de salida

- Todos los servicios críticos en `healthy`
- PostgreSQL restaurado con migraciones alineadas
- Object storage accesible y con paths esperados
- Al menos un flujo `dataset -> ontology -> app` válido
- Al menos un `fleet sync` de prueba bloqueado o permitido por gates según lo esperado
