# 🏗️ Checklist de Evaluación de Arquitectura — Monorepo con Microservicios

Evalúa el proyecto actual de forma exhaustiva siguiendo cada sección. Para cada punto responde con:
- ✅ Cumple
- ⚠️ Cumple parcialmente (explica qué falta)
- ❌ No cumple (explica el impacto y sugiere mejora)
- 🔘 No aplica

---

## 1. ESTRUCTURA DEL MONOREPO

### 1.1 Organización raíz
- [ ] ¿La raíz del monorepo tiene una estructura clara y documentada?
- [ ] ¿Hay separación evidente entre servicios, librerías compartidas, configuración e infraestructura?
- [ ] ¿Se sigue una convención de naming consistente para carpetas y paquetes?
- [ ] ¿La estructura permite descubrir servicios y librerías sin documentación adicional?

Estructura esperada (ejemplo):
```
monorepo/
├── services/           # Microservicios
│   ├── auth/
│   ├── billing/
│   ├── notifications/
│   └── users/
├── packages/           # Librerías compartidas
│   ├── shared-types/
│   ├── logger/
│   ├── db-client/
│   └── event-bus/
├── infra/              # IaC, Dockerfiles, Helm charts
├── tools/              # Scripts, generadores, CLI internos
├── docs/               # Documentación global, ADRs
├── .github/            # CI/CD workflows
├── turbo.json / nx.json / pnpm-workspace.yaml
└── README.md
```

### 1.2 Herramienta de gestión del monorepo
- [ ] ¿Se usa una herramienta de monorepo (Nx, Turborepo, Bazel, Lerna, Rush)?
- [ ] ¿Está configurada correctamente la resolución de dependencias entre paquetes?
- [ ] ¿Hay caching de builds configurado (local y/o remoto)?
- [ ] ¿Los comandos del monorepo están documentados en el README raíz?
- [ ] ¿Se pueden ejecutar tareas de forma selectiva por servicio/paquete afectado?

### 1.3 Gestión de dependencias
- [ ] ¿Hay un lockfile único y consistente en la raíz?
- [ ] ¿Se usa hoisting controlado de dependencias (no phantom dependencies)?
- [ ] ¿Las versiones de dependencias compartidas están alineadas entre servicios?
- [ ] ¿Hay política clara sobre cuándo una dependencia debe ser compartida vs. local?
- [ ] ¿Se usan workspaces nativos (pnpm/yarn/npm workspaces, Go workspaces, Cargo workspaces)?

---

## 2. ARQUITECTURA DE MICROSERVICIOS

### 2.1 Boundaries y responsabilidades
- [ ] ¿Cada microservicio tiene un dominio de negocio claramente definido (Bounded Context)?
- [ ] ¿Los servicios son desplegables de forma independiente?
- [ ] ¿Cada servicio tiene su propia base de datos / esquema (Database per Service)?
- [ ] ¿No hay acceso directo a la BD de otro servicio?
- [ ] ¿El tamaño de cada servicio es justificable? (ni nano-servicios ni monolitos disfrazados)
- [ ] ¿Se puede explicar la responsabilidad de cada servicio en una frase?
- [ ] ¿Hay un mapa de servicios / diagrama de contexto documentado?

### 2.2 Comunicación entre servicios
- [ ] ¿Está definido claramente qué comunicación es síncrona y cuál asíncrona?
- [ ] ¿La comunicación síncrona usa protocolos bien definidos (REST con OpenAPI, gRPC con .proto)?
- [ ] ¿La comunicación asíncrona usa un message broker (Kafka, RabbitMQ, NATS, SQS)?
- [ ] ¿Los eventos/mensajes tienen esquemas versionados y validados?
- [ ] ¿Se evitan cadenas largas de llamadas síncronas entre servicios (orquestación excesiva)?
- [ ] ¿Hay un patrón definido para sagas / transacciones distribuidas donde aplique?
- [ ] ¿Los contratos entre servicios están definidos y testeados (Contract Testing)?
- [ ] ¿Se usa un API Gateway o BFF (Backend for Frontend) donde corresponda?

### 2.3 Consistencia y datos
- [ ] ¿Se acepta y maneja explícitamente la consistencia eventual?
- [ ] ¿Hay estrategia para resolver conflictos de datos entre servicios?
- [ ] ¿Los eventos de dominio tienen semántica clara (Event Carried State Transfer, Event Notification)?
- [ ] ¿Hay idempotencia en los consumidores de mensajes/eventos?
- [ ] ¿Se maneja correctamente el ordenamiento de eventos donde importa?
- [ ] ¿Existe dead letter queue (DLQ) para mensajes fallidos?

---

## 3. LIBRERÍAS Y CÓDIGO COMPARTIDO

### 3.1 Paquetes compartidos
- [ ] ¿Las librerías compartidas están en packages/ (o equivalente) con su propio package.json/go.mod?
- [ ] ¿Cada librería compartida tiene una responsabilidad única y clara?
- [ ] ¿Las librerías compartidas tienen su propio README con API documentada?
- [ ] ¿Los cambios en librerías compartidas disparan tests de los servicios que las consumen?
- [ ] ¿Se evita el "shared kernel" excesivo que acopla todos los servicios?

### 3.2 Tipos y contratos compartidos
- [ ] ¿Los DTOs / tipos compartidos entre servicios están centralizados en un paquete de tipos?
- [ ] ¿Los esquemas de eventos están definidos en un paquete compartido con versionado?
- [ ] ¿Hay generación automática de tipos desde esquemas (protobuf, OpenAPI, JSON Schema)?
- [ ] ¿Los tipos compartidos son solo de datos (sin lógica de negocio)?

### 3.3 Prevención de acoplamiento
- [ ] ¿Hay reglas que impidan imports directos entre servicios?
- [ ] ¿Se usa alguna herramienta para validar boundaries (Nx module boundaries, eslint-plugin-boundaries, ArchUnit)?
- [ ] ¿Cambiar un servicio NO requiere cambiar otro servicio simultáneamente?
- [ ] ¿Se mide el fan-in/fan-out de cada paquete compartido?

---

## 4. ESTRUCTURA INTERNA DE CADA MICROSERVICIO

### 4.1 Arquitectura interna
- [ ] ¿Cada servicio sigue una arquitectura interna consistente (Clean, Hexagonal, Vertical Slices)?
- [ ] ¿Todos los servicios siguen la MISMA estructura interna? (o hay justificación para diferencias)
- [ ] ¿Hay separación entre dominio, aplicación e infraestructura dentro de cada servicio?

Estructura interna esperada por servicio:
```
services/users/
├── src/
│   ├── domain/          # Entidades, Value Objects, interfaces de repositorio
│   │   ├── entities/
│   │   ├── value-objects/
│   │   ├── events/
│   │   └── ports/       # Interfaces (repository, event publisher)
│   ├── application/     # Use Cases / Commands / Queries
│   │   ├── commands/
│   │   ├── queries/
│   │   └── handlers/
│   ├── infrastructure/  # Implementaciones concretas
│   │   ├── database/
│   │   ├── messaging/
│   │   ├── http-client/
│   │   └── adapters/
│   └── interface/       # Puntos de entrada
│       ├── http/        # Controllers / Routes
│       ├── grpc/
│       └── consumers/   # Event consumers
├── tests/
│   ├── unit/
│   ├── integration/
│   └── e2e/
├── Dockerfile
├── package.json
├── tsconfig.json
└── README.md
```

### 4.2 Principios SOLID
- [ ] **S** — ¿Las clases/módulos dentro de cada servicio tienen una sola responsabilidad?
- [ ] **O** — ¿Se puede extender comportamiento sin modificar código existente?
- [ ] **L** — ¿Las implementaciones son sustituibles por sus abstracciones?
- [ ] **I** — ¿Las interfaces son pequeñas y específicas?
- [ ] **D** — ¿El dominio depende de abstracciones, no de implementaciones concretas?
- [ ] ¿Se aplica DRY dentro de cada servicio sin abstracciones prematuras?
- [ ] ¿Se aplica KISS? ¿Hay sobre-ingeniería visible?

### 4.3 Gestión de estado y datos por servicio
- [ ] ¿Cada servicio es dueño exclusivo de sus datos?
- [ ] ¿Hay una capa de acceso a datos clara (Repository pattern)?
- [ ] ¿Las migraciones están versionadas y son reversibles?
- [ ] ¿Los modelos de dominio están separados de los modelos de persistencia y API?
- [ ] ¿Se evita el estado mutable compartido dentro del servicio?
- [ ] ¿Hay validación de datos en la capa de dominio?

---

## 5. TESTEABILIDAD

### 5.1 Tests por servicio
- [ ] ¿Cada servicio tiene tests unitarios para su lógica de dominio?
- [ ] ¿Cada servicio tiene tests de integración para adaptadores (BD, messaging)?
- [ ] ¿Los tests unitarios se ejecutan sin infraestructura externa?
- [ ] ¿Los tests de integración usan contenedores (Testcontainers, docker-compose)?
- [ ] ¿La lógica de negocio se puede testear sin mocks complejos?
- [ ] ¿Hay cobertura de tests mínima definida y aplicada en CI?

### 5.2 Tests entre servicios
- [ ] ¿Hay Contract Tests entre servicios (Pact, Schema Registry)?
- [ ] ¿Hay tests end-to-end para los flujos críticos que cruzan servicios?
- [ ] ¿Se puede levantar un subconjunto de servicios localmente para testing?
- [ ] ¿Los tests E2E no son frágiles ni lentos?
- [ ] ¿Hay smoke tests post-deploy?

### 5.3 Tests de librerías compartidas
- [ ] ¿Las librerías compartidas tienen su propia suite de tests?
- [ ] ¿Un cambio en una librería dispara automáticamente los tests de los consumidores?

---

## 6. MANEJO DE ERRORES Y RESILIENCIA

### 6.1 Errores dentro de cada servicio
- [ ] ¿Hay una estrategia consistente de manejo de errores por servicio?
- [ ] ¿Se usan tipos de error del dominio (no solo excepciones genéricas)?
- [ ] ¿Los errores se propagan correctamente entre capas internas?
- [ ] ¿Se evitan los catch vacíos / swallow silencioso?
- [ ] ¿Los mensajes de error son útiles para debugging?

### 6.2 Resiliencia entre servicios
- [ ] ¿Hay circuit breakers para llamadas a otros servicios?
- [ ] ¿Hay retry con backoff exponencial para operaciones recuperables?
- [ ] ¿Hay timeouts configurados en todas las llamadas externas (HTTP, gRPC, BD)?
- [ ] ¿Hay bulkheads para aislar fallos (un servicio caído no tumba todo)?
- [ ] ¿Hay fallbacks / degradación graceful cuando un servicio no está disponible?
- [ ] ¿Se maneja correctamente la presión inversa (backpressure) en colas?
- [ ] ¿El sistema se recupera automáticamente cuando un servicio vuelve?

### 6.3 Patrones de resiliencia distribuida
- [ ] ¿Se implementa idempotencia en todas las operaciones que lo requieren?
- [ ] ¿Hay compensación / rollback para sagas/transacciones distribuidas?
- [ ] ¿Se maneja el caso de mensajes duplicados?
- [ ] ¿Se maneja el caso de mensajes fuera de orden?

---

## 7. OBSERVABILIDAD

### 7.1 Logging
- [ ] ¿Hay logging estructurado (JSON) consistente entre todos los servicios?
- [ ] ¿Se usa una librería de logging compartida (paquete interno)?
- [ ] ¿Los logs tienen niveles apropiados y consistentes?
- [ ] ¿Se evita loguear información sensible (PII, tokens, passwords)?
- [ ] ¿Los logs se centralizan en una plataforma (ELK, Loki, CloudWatch)?

### 7.2 Distributed Tracing
- [ ] ¿Hay tracing distribuido implementado (OpenTelemetry, Jaeger, Zipkin)?
- [ ] ¿Cada request tiene un correlation ID / trace ID que cruza servicios?
- [ ] ¿El trace ID se propaga tanto en llamadas síncronas como asíncronas?
- [ ] ¿Se pueden visualizar las trazas completas de un flujo multi-servicio?
- [ ] ¿Los spans tienen atributos útiles (user_id, order_id, etc.)?

### 7.3 Métricas y alertas
- [ ] ¿Cada servicio expone métricas (Prometheus, StatsD)?
- [ ] ¿Se miden los 4 Golden Signals (latencia, tráfico, errores, saturación)?
- [ ] ¿Hay health checks (liveness + readiness) en cada servicio?
- [ ] ¿Hay dashboards por servicio y un dashboard global del sistema?
- [ ] ¿Hay alertas configuradas para fallos críticos?
- [ ] ¿Se mide la latencia P50, P95, P99 de cada servicio?

---

## 8. CI/CD Y BUILD

### 8.1 Pipeline
- [ ] ¿Hay pipelines de CI/CD definidos y funcionales?
- [ ] ¿Los builds son incrementales (solo se construye lo afectado por el cambio)?
- [ ] ¿Se usa el grafo de dependencias del monorepo para determinar qué testear/deployar?
- [ ] ¿Hay caching efectivo en CI (dependencias, builds previos, layers de Docker)?
- [ ] ¿El pipeline de un servicio individual es rápido (<10 min idealmente)?
- [ ] ¿Hay paralelización de tests y builds en CI?

### 8.2 Deploy
- [ ] ¿Cada servicio se puede desplegar de forma independiente?
- [ ] ¿Hay deploy automatizado a staging/producción?
- [ ] ¿Se usa versionado semántico o basado en commits para cada servicio?
- [ ] ¿Hay estrategia de rollback automatizada?
- [ ] ¿Se usan rolling deployments, blue/green o canary?
- [ ] ¿Hay feature flags para desplegar código inactivo de forma segura?

### 8.3 Calidad de código
- [ ] ¿Hay linter/formatter compartido y aplicado consistentemente?
- [ ] ¿Hay type checking estricto?
- [ ] ¿Las reglas de lint son las mismas para todos los servicios?
- [ ] ¿Hay pre-commit hooks o checks en CI que impidan código que no cumple?
- [ ] ¿Hay análisis estático de seguridad (SAST) en el pipeline?

---

## 9. INFRAESTRUCTURA Y CONTENEDORIZACIÓN

### 9.1 Docker
- [ ] ¿Cada servicio tiene un Dockerfile optimizado (multi-stage, capas cacheables)?
- [ ] ¿Las imágenes base son consistentes entre servicios?
- [ ] ¿Las imágenes son ligeras (Alpine, distroless)?
- [ ] ¿Los Dockerfiles evitan copiar el monorepo entero al contexto de build?
- [ ] ¿Hay un docker-compose para levantar todo el stack local?
- [ ] ¿Se puede levantar un subconjunto de servicios con sus dependencias?

### 9.2 Orquestación
- [ ] ¿Hay IaC definido (Terraform, Pulumi, CDK, Helm charts)?
- [ ] ¿El IaC está versionado en el mismo monorepo?
- [ ] ¿Cada servicio define sus recursos de infra necesarios?
- [ ] ¿Hay separación de infra por entorno (dev, staging, prod)?
- [ ] ¿Se usan namespaces o aislamiento lógico entre servicios en orquestador?

### 9.3 Service Mesh / Networking
- [ ] ¿Hay service discovery configurado (DNS interno, Consul, Kubernetes services)?
- [ ] ¿Se usa mTLS o algún mecanismo de autenticación entre servicios?
- [ ] ¿Hay rate limiting entre servicios?
- [ ] ¿El tráfico entre servicios es observable?

---

## 10. SEGURIDAD

### 10.1 Seguridad por servicio
- [ ] ¿La validación de entrada ocurre en la capa de interfaz de cada servicio?
- [ ] ¿Hay sanitización de datos antes de persistir?
- [ ] ¿Los endpoints tienen autenticación y autorización?
- [ ] ¿Hay protección contra inyecciones (SQL, NoSQL, XSS)?
- [ ] ¿Las dependencias de cada servicio están libres de CVEs conocidos?

### 10.2 Seguridad entre servicios
- [ ] ¿Las llamadas entre servicios están autenticadas (tokens internos, mTLS)?
- [ ] ¿No se confía ciegamente en peticiones internas (zero trust)?
- [ ] ¿Hay un servicio centralizado de auth o se propagan tokens correctamente?
- [ ] ¿Los permisos/roles se validan en cada servicio (no solo en el gateway)?

### 10.3 Secretos y configuración
- [ ] ¿Los secretos están en un vault (Vault, AWS Secrets Manager, GCP Secret Manager)?
- [ ] ¿Los secretos NUNCA están en código, variables de entorno del repo o config files comiteados?
- [ ] ¿Hay rotación de secretos automatizada?
- [ ] ¿Cada servicio tiene acceso solo a los secretos que necesita (least privilege)?

---

## 11. CONFIGURACIÓN Y ENTORNO

- [ ] ¿La configuración de cada servicio está separada del código?
- [ ] ¿Hay validación de configuración al arrancar cada servicio?
- [ ] ¿Hay configuraciones por entorno bien definidas?
- [ ] ¿Existe documentación de todas las variables de entorno requeridas por servicio?
- [ ] ¿Hay valores por defecto sensatos para desarrollo local?
- [ ] ¿La configuración compartida (URLs de servicios, etc.) se gestiona centralmente?

---

## 12. DOCUMENTACIÓN

### 12.1 Documentación global
- [ ] ¿El README raíz explica la arquitectura general, cómo levantar todo y cómo contribuir?
- [ ] ¿Hay un diagrama de arquitectura actualizado del sistema completo?
- [ ] ¿Hay ADRs (Architecture Decision Records) para decisiones clave?
- [ ] ¿Hay un mapa de servicios con sus responsabilidades y dependencias?
- [ ] ¿Hay un runbook / playbook para incidentes comunes?
- [ ] ¿Hay guía de "cómo crear un nuevo microservicio" (template/scaffold)?

### 12.2 Documentación por servicio
- [ ] ¿Cada servicio tiene su propio README con setup, API y decisiones locales?
- [ ] ¿La API de cada servicio está documentada (OpenAPI, gRPC docs, AsyncAPI)?
- [ ] ¿Los eventos que publica/consume cada servicio están documentados?
- [ ] ¿Hay documentación de los flujos de negocio que cruzan servicios?

### 12.3 Onboarding
- [ ] ¿Un dev nuevo puede levantar el proyecto en <30 minutos?
- [ ] ¿Hay un script/comando único para setup inicial?
- [ ] ¿La documentación de onboarding está probada y actualizada?

---

## 13. RENDIMIENTO Y ESCALABILIDAD

- [ ] ¿Cada servicio puede escalar horizontalmente de forma independiente?
- [ ] ¿Hay auto-scaling configurado basado en métricas?
- [ ] ¿Los servicios con alta carga están separados de los de baja carga?
- [ ] ¿Se usa caching distribuido donde es apropiado (Redis, Memcached)?
- [ ] ¿Las consultas a BD están optimizadas (índices, paginación, projections)?
- [ ] ¿Hay connection pooling para BD y servicios externos?
- [ ] ¿Los event consumers pueden paralelizar procesamiento?
- [ ] ¿Hay load testing / benchmarks de los flujos críticos?
- [ ] ¿Se manejan correctamente los picos de tráfico?

---

## 14. DEVELOPER EXPERIENCE (DX)

- [ ] ¿Los tiempos de build en local son razonables (<2 min por servicio)?
- [ ] ¿Hay hot-reload / watch mode para desarrollo local?
- [ ] ¿Se puede debuggear un servicio individual fácilmente?
- [ ] ¿Hay generadores/templates para crear nuevos servicios, endpoints, eventos?
- [ ] ¿Los errores de build son claros y accionables?
- [ ] ¿Hay IDE support adecuado (imports, autocompletado entre paquetes)?
- [ ] ¿Los PRs ejecutan solo los checks relevantes al cambio?
- [ ] ¿Hay seed data / fixtures para desarrollo local?

---

## 📊 RESUMEN EJECUTIVO

Al finalizar, genera:

1. **Puntuación por sección** (0-10) y puntuación global ponderada
2. **Mapa de madurez por servicio** — tabla comparando la madurez de cada servicio

| Servicio | Tests | Docs | Observabilidad | Resiliencia | Seguridad | Score |
|----------|-------|------|----------------|-------------|-----------|-------|
| auth     | 8/10  | 7/10 | 9/10           | 8/10        | 9/10      | 8.2   |
| billing  | 5/10  | 3/10 | 4/10           | 6/10        | 7/10      | 5.0   |
| ...      |       |      |                |             |           |       |

3. **Top 5 fortalezas** de la arquitectura actual
4. **Top 5 debilidades críticas** ordenadas por impacto en producción
5. **Top 5 riesgos** — problemas que aún no han explotado pero lo harán
6. **Deuda técnica** categorizada:
   - 🔴 Crítica (bloquea escalabilidad o causa incidentes)
   - 🟡 Media (ralentiza desarrollo)
   - 🟢 Baja (nice to have)
7. **Plan de acción priorizado**:
   - Quick wins (< 1 semana)
   - Mejoras a corto plazo (1-4 semanas)
   - Inversiones estratégicas (1-3 meses)
8. **Diagrama de la arquitectura actual** vs. **arquitectura ideal recomendada**
9. **Servicios candidatos a fusión** (si hay nano-servicios innecesarios)
10. **Servicios candidatos a división** (si hay monolitos disfrazados)

---

### Instrucciones para Claude Code:
- Recorre TODO el codebase antes de responder: raíz, cada servicio, cada paquete compartido
- Verifica la herramienta de monorepo y su configuración (turbo.json, nx.json, etc.)
- Lee los Dockerfiles, docker-compose, Helm charts y CI/CD workflows
- Inspecciona los schemas de eventos/mensajes y contratos entre servicios
- Verifica imports entre servicios para detectar acoplamiento ilegal
- Revisa las migraciones de BD de cada servicio
- No asumas; verifica cada punto leyendo el código
- Da ejemplos concretos de archivos/líneas cuando encuentres problemas
- Si un punto no aplica, márcalo como 🔘 y explica por qué
- Sé honesto y directo: la utilidad depende de la precisión
