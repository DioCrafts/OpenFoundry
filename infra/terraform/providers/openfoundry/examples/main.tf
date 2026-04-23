terraform {
  required_version = ">= 1.7.0"

  required_providers {
    openfoundry = {
      source  = "openfoundry/openfoundry"
      version = "~> 0.1"
    }
  }
}

provider "openfoundry" {
  api_url   = "https://platform.openfoundry.local"
  token     = var.openfoundry_token
  workspace = "production"
}

resource "openfoundry_repository_integration" "github_widget_kit" {
  repository_id       = "0196839d-d210-7f8c-8a1d-7ab001030001"
  provider            = "github"
  external_project    = "foundry-widget-kit"
  sync_mode           = "bidirectional_mirror"
  ci_trigger_strategy = "github_actions"
}

resource "openfoundry_nexus_peer" "acme_health" {
  slug          = "acme-health"
  endpoint_url  = "https://nexus.acme-health.example"
  auth_mode     = "mtls+jwt"
  shared_scopes = ["claims", "audit"]
}

resource "openfoundry_audit_policy" "pii_retention" {
  name           = "PII retention"
  classification = "pii"
  retention_days = 365
  purge_mode     = "redact-then-retain-hash"
}

variable "openfoundry_token" {
  type      = string
  sensitive = true
}