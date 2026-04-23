{{- define "open-foundry.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "open-foundry.fullname" -}}
{{- printf "%s-%s" (include "open-foundry.name" .root) .serviceName | trunc 63 | trimSuffix "-" -}}
{{- end -}}