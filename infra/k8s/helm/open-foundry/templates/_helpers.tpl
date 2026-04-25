{{- define "open-foundry.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "open-foundry.fullname" -}}
{{- printf "%s-%s" (include "open-foundry.name" .root) .serviceName | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "open-foundry.namespace" -}}
{{- default .root.Release.Namespace .root.Values.namespaceOverride -}}
{{- end -}}

{{- define "open-foundry.selectorLabels" -}}
app.kubernetes.io/name: {{ include "open-foundry.name" .root }}
app.kubernetes.io/instance: {{ .root.Release.Name }}
app.kubernetes.io/component: {{ .serviceName }}
{{- end -}}

{{- define "open-foundry.commonLabels" -}}
{{ include "open-foundry.selectorLabels" . }}
app.kubernetes.io/managed-by: {{ .root.Release.Service }}
helm.sh/chart: {{ printf "%s-%s" .root.Chart.Name .root.Chart.Version | replace "+" "_" }}
{{- range $labelKey, $labelValue := .root.Values.global.labels }}
{{ $labelKey }}: {{ $labelValue | quote }}
{{- end }}
{{- end -}}

{{- define "open-foundry.serviceAccountName" -}}
{{- if .root.Values.serviceAccount.create -}}
{{- default (include "open-foundry.name" .root) .root.Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .root.Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}

{{- define "open-foundry.serviceHost" -}}
{{- printf "%s.%s.svc.cluster.local" (include "open-foundry.fullname" (dict "root" .root "serviceName" .serviceName)) (include "open-foundry.namespace" (dict "root" .root)) -}}
{{- end -}}

{{- define "open-foundry.serviceUrl" -}}
{{- printf "http://%s:%v" (include "open-foundry.serviceHost" .) .port -}}
{{- end -}}
