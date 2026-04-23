# 🏗️ Checklist de Evaluación de Arquitectura de Software

Evalúa el proyecto actual de forma exhaustiva siguiendo cada sección. Para cada punto responde con:
- ✅ Cumple
- ⚠️ Cumple parcialmente (explica qué falta)
- ❌ No cumple (explica el impacto y sugiere mejora)
- 🔘 No aplica

---

## 1. ESTRUCTURA Y ORGANIZACIÓN

- [ ] ¿Existe una estructura de carpetas clara y consistente?
- [ ] ¿Se puede identificar qué arquitectura sigue (Clean, Hexagonal, MVC, Vertical Slices, etc.)?
- [ ] ¿La estructura refleja el dominio del negocio y no solo la tecnología? (Screaming Architecture)
- [ ] ¿Los módulos/paquetes tienen responsabilidades bien definidas y acotadas?
- [ ] ¿Hay separación clara entre código de negocio, infraestructura y presentación?
- [ ] ¿La estructura es navegable? ¿Un dev nuevo podría encontrar algo sin guía?
- [ ] ¿Hay archivos o carpetas "cajón de sastre" (utils/, helpers/, misc/) con demasiada responsabilidad?

## 2. DEPENDENCIAS Y ACOPLAMIENTO

- [ ] ¿Las dependencias fluyen en una sola dirección? (de afuera hacia adentro)
- [ ] ¿El core/dominio depende de frameworks o librerías externas?
- [ ] ¿Se usa inversión de dependencias (interfaces/puertos) para desacoplar capas?
- [ ] ¿Se podrían reemplazar la BD, el framework web o la UI sin tocar la lógica de negocio?
- [ ] ¿El archivo de dependencias (package.json, requirements.txt, etc.) tiene dependencias innecesarias o desactualizadas?
- [ ] ¿Hay dependencias circulares entre módulos?
- [ ] ¿Los imports entre módulos respetan los límites arquitectónicos?

## 3. PRINCIPIOS SOLID Y DISEÑO

- [ ] **S** — ¿Las clases/módulos tienen una sola responsabilidad?
- [ ] **O** — ¿Se puede extender comportamiento sin modificar código existente?
- [ ] **L** — ¿Las implementaciones son sustituibles por sus abstracciones?
- [ ] **I** — ¿Las interfaces son pequeñas y específicas (no interfaces "gordas")?
- [ ] **D** — ¿Las capas de alto nivel dependen de abstracciones, no de implementaciones concretas?
- [ ] ¿Se aplica DRY sin caer en abstracciones prematuras?
- [ ] ¿Se aplica KISS? ¿Hay sobre-ingeniería visible?
- [ ] ¿Se aplica YAGNI? ¿Hay código/abstracciones para funcionalidades que no existen?

## 4. TESTEABILIDAD

- [ ] ¿Existe una suite de tests? ¿Qué porcentaje de cobertura tiene?
- [ ] ¿Hay tests unitarios para la lógica de negocio/dominio?
- [ ] ¿Hay tests de integración para las capas externas?
- [ ] ¿Los tests son independientes entre sí (no comparten estado)?
- [ ] ¿Se pueden ejecutar los tests sin depender de servicios externos (BD, APIs)?
- [ ] ¿La lógica de negocio se puede testear sin mocks complejos?
- [ ] ¿Los tests documentan el comportamiento esperado del sistema?
- [ ] ¿Hay tests end-to-end para los flujos críticos?

## 5. MANEJO DE ERRORES Y RESILIENCIA

- [ ] ¿Hay una estrategia consistente de manejo de errores?
- [ ] ¿Se usan tipos de error personalizados del dominio (no solo excepciones genéricas)?
- [ ] ¿Los errores se propagan correctamente entre capas?
- [ ] ¿Se evita el "swallow" silencioso de errores (catch vacíos)?
- [ ] ¿Hay manejo de errores en las fronteras del sistema (API, BD, archivos)?
- [ ] ¿Existe retry logic / circuit breaker donde aplique?
- [ ] ¿Los mensajes de error son útiles para debugging?

## 6. OBSERVABILIDAD Y DEBUGGING

- [ ] ¿Hay logging estructurado y consistente?
- [ ] ¿Los logs tienen niveles apropiados (debug, info, warn, error)?
- [ ] ¿Se puede rastrear una petición/operación completa a través de los logs?
- [ ] ¿Hay métricas o health checks implementados?
- [ ] ¿Los errores en producción serían fácilmente rastreables hasta su origen?
- [ ] ¿Se evita loguear información sensible (passwords, tokens, PII)?

## 7. CONFIGURACIÓN Y ENTORNO

- [ ] ¿La configuración está separada del código (env vars, config files)?
- [ ] ¿Hay validación de configuración al iniciar la aplicación?
- [ ] ¿Los secretos están fuera del repositorio?
- [ ] ¿Hay configuraciones por entorno (dev, staging, prod)?
- [ ] ¿Existe un .env.example o documentación de variables requeridas?
- [ ] ¿Hay valores por defecto sensatos para desarrollo local?

## 8. SEGURIDAD

- [ ] ¿La validación de entrada ocurre en las fronteras del sistema?
- [ ] ¿Hay sanitización de datos antes de persistir o renderizar?
- [ ] ¿La autenticación y autorización están centralizadas?
- [ ] ¿Se siguen las prácticas de seguridad del framework/lenguaje?
- [ ] ¿Hay protección contra inyecciones (SQL, XSS, CSRF)?
- [ ] ¿Los endpoints/rutas tienen control de acceso apropiado?
- [ ] ¿Las dependencias tienen vulnerabilidades conocidas?

## 9. RENDIMIENTO Y ESCALABILIDAD

- [ ] ¿Hay consultas N+1 o patrones de acceso a datos ineficientes?
- [ ] ¿Se usa caching donde es apropiado?
- [ ] ¿Las operaciones costosas son asíncronas donde aplique?
- [ ] ¿Hay índices apropiados en la base de datos?
- [ ] ¿El sistema podría escalar horizontalmente si fuera necesario?
- [ ] ¿Hay cuellos de botella evidentes (archivos monolíticos, loops costosos)?
- [ ] ¿Se manejan correctamente las conexiones a servicios externos (pools, timeouts)?

## 10. DOCUMENTACIÓN Y MANTENIBILIDAD

- [ ] ¿Hay un README con instrucciones claras para levantar el proyecto?
- [ ] ¿Las decisiones arquitectónicas están documentadas (ADRs)?
- [ ] ¿El código se auto-documenta con nombres claros y expresivos?
- [ ] ¿Los comentarios explican el "por qué", no el "qué"?
- [ ] ¿Hay documentación de la API (OpenAPI/Swagger, GraphQL schema)?
- [ ] ¿Existe guía de contribución o estándares de código?
- [ ] ¿El onboarding de un nuevo desarrollador sería sencillo?

## 11. CI/CD Y CALIDAD DE CÓDIGO

- [ ] ¿Hay pipeline de CI que ejecute tests automáticamente?
- [ ] ¿Hay linter/formatter configurado y aplicado consistentemente?
- [ ] ¿Hay type checking estricto (TypeScript strict, mypy, etc.)?
- [ ] ¿El código pasa todas las verificaciones de CI actuales?
- [ ] ¿Hay análisis estático de código (SonarQube, ESLint rules avanzadas)?
- [ ] ¿El proceso de deploy es automatizado y reproducible?

## 12. GESTIÓN DE ESTADO Y DATOS

- [ ] ¿El flujo de datos es predecible y rastreable?
- [ ] ¿Hay una capa clara de acceso a datos (repository pattern, DAL)?
- [ ] ¿Las migraciones de BD están versionadas y son reversibles?
- [ ] ¿Se evita el estado mutable compartido?
- [ ] ¿Los modelos de dominio están separados de los modelos de persistencia y de API?
- [ ] ¿Hay validación de datos en la capa de dominio (no solo en la BD)?

---

## 📊 RESUMEN EJECUTIVO

Al finalizar, genera:

1. **Puntuación por sección** (0-10) y puntuación global
2. **Top 5 fortalezas** de la arquitectura actual
3. **Top 5 debilidades críticas** ordenadas por impacto
4. **Deuda técnica identificada** con estimación de esfuerzo (bajo/medio/alto)
5. **Plan de acción priorizado** con quick wins y mejoras a largo plazo
6. **Arquitectura recomendada** si la actual no es la adecuada para el caso de uso
7. **Diagrama simplificado** de la arquitectura actual vs. la ideal

---

### Instrucciones para Claude Code:
- Recorre TODO el codebase antes de responder
- No asumas; verifica cada punto leyendo el código
- Da ejemplos concretos de archivos/líneas cuando encuentres problemas
- Si un punto no aplica al tipo de proyecto, márcalo como 🔘 y explica por qué
- Sé honesto y directo: la utilidad de esta evaluación depende de su precisión
