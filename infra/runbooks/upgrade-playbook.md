# Upgrade Playbook

Fecha: 25 de abril de 2026

## Objetivo

Ejecutar upgrades repetibles de OpenFoundry con validación previa, ventana de mantenimiento y rollback explícito.

## Preflight

- Validar Terraform/Helm del entorno
- Confirmar compatibilidad de migraciones
- Generar backup lógico de PostgreSQL
- Generar backup de buckets críticos
- Revisar gates de promotion en fleets sensibles

## Estrategia recomendada

1. `canary` en una deployment cell
2. Validación de métricas y smoke checks
3. Promoción a `stable`
4. Rollout al resto de cells dentro de maintenance window

## Rollback

- Revertir imagen o chart version
- Restaurar DB solo si hubo cambio destructivo o corrupción de datos
- Rehabilitar reconciliadores una vez establecida la versión anterior

## Evidencias mínimas

- Commit o tag desplegado
- Hora de inicio y fin
- Resultado de smoke checks
- Estado de gates
- Versiones previas y nuevas
