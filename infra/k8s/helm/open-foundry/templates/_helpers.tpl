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
openfoundry.io/cloud: {{ .root.Values.global.deploymentFabric.cloud | quote }}
openfoundry.io/region: {{ .root.Values.global.deploymentFabric.region | quote }}
openfoundry.io/cell: {{ .root.Values.global.deploymentFabric.cell | quote }}
openfoundry.io/environment: {{ .root.Values.global.deploymentFabric.environment | quote }}
{{- if .root.Values.global.deploymentFabric.geoRestrictions.enabled }}
openfoundry.io/residency: {{ default "geo-restricted" .root.Values.global.deploymentFabric.geoRestrictions.residencyLabel | quote }}
{{- end }}
{{- if .root.Values.apollo.enabled }}
openfoundry.io/apollo: "enabled"
{{- end }}
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

{{- define "open-foundry.platformProfileConfigMapName" -}}
{{- printf "%s-platform-profile" (include "open-foundry.name" .root) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "open-foundry.image" -}}
{{- if .root.Values.global.imageRegistry -}}
{{- printf "%s/%s:%s" .root.Values.global.imageRegistry .repository .tag -}}
{{- else -}}
{{- printf "%s:%s" .repository .tag -}}
{{- end -}}
{{- end -}}

{{- define "open-foundry.apolloGatewayUrl" -}}
{{- if .root.Values.apollo.gatewayUrl -}}
{{- .root.Values.apollo.gatewayUrl -}}
{{- else -}}
{{- include "open-foundry.serviceUrl" (dict "root" .root "serviceName" "gateway" "port" (index .root.Values.services "gateway").port) -}}
{{- end -}}
{{- end -}}
