variable "name" {
  type        = string
  description = "Logical CDN profile name."
}

variable "asset_origin_host" {
  type        = string
  description = "Origin hostname for static application assets."
}

variable "tile_origin_host" {
  type        = string
  description = "Origin hostname for geospatial tile traffic."
}

variable "custom_domains" {
  type        = list(string)
  description = "Custom domains exposed by the CDN edge."
  default     = []
}

variable "regions" {
  type        = list(string)
  description = "Preferred edge regions or POP groups."
  default     = ["global"]
}

variable "price_class" {
  type        = string
  description = "Edge footprint / price class label."
  default     = "global"
}

variable "static_asset_ttl_seconds" {
  type        = number
  description = "TTL for immutable application assets."
  default     = 86400
}

variable "tile_ttl_seconds" {
  type        = number
  description = "TTL for geospatial vector tiles."
  default     = 300
}

variable "enable_compression" {
  type        = bool
  description = "Enable Brotli/Gzip edge compression for static assets."
  default     = true
}

variable "enable_http3" {
  type        = bool
  description = "Enable HTTP/3 at the edge."
  default     = true
}

variable "waf_enabled" {
  type        = bool
  description = "Whether the CDN profile should be attached to a WAF policy."
  default     = true
}

variable "allowed_countries" {
  type        = list(string)
  description = "Optional ISO country allowlist enforced at the edge."
  default     = []
}

variable "geo_restriction_mode" {
  type        = string
  description = "allow_all, whitelist, or sovereign_only."
  default     = "allow_all"
}

variable "origin_residency_region" {
  type        = string
  description = "Preferred origin-residency region for regulated traffic."
  default     = ""
}

variable "airgapped_origin" {
  type        = bool
  description = "Whether the backing origin is served from an air-gapped or private network posture."
  default     = false
}

variable "stale_while_revalidate_seconds" {
  type        = number
  description = "Serve stale responses while fetching fresh content from origin."
  default     = 60
}

variable "stale_if_error_seconds" {
  type        = number
  description = "Serve stale responses on origin failure."
  default     = 600
}
