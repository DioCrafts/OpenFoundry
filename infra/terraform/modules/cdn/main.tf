terraform {
  required_version = ">= 1.7.0"
}

locals {
  asset_route = {
    origin_host       = var.asset_origin_host
    cache_ttl_seconds = var.static_asset_ttl_seconds
    compression       = var.enable_compression
    cache_key_headers = ["Accept-Encoding", "If-None-Match"]
    path_patterns     = ["/assets/*", "/generated/*", "/_app/*"]
  }

  tile_route = {
    origin_host       = var.tile_origin_host
    cache_ttl_seconds = var.tile_ttl_seconds
    compression       = false
    cache_key_headers = ["Accept", "If-None-Match"]
    path_patterns     = ["/tiles/*", "/api/v1/geospatial/tiles/*"]
  }
}

resource "terraform_data" "edge_cache_profile" {
  input = {
    name                          = var.name
    custom_domains                = var.custom_domains
    regions                       = var.regions
    price_class                   = var.price_class
    waf_enabled                   = var.waf_enabled
    allowed_countries             = var.allowed_countries
    geo_restriction_mode          = var.geo_restriction_mode
    origin_residency_region       = var.origin_residency_region
    airgapped_origin              = var.airgapped_origin
    enable_http3                  = var.enable_http3
    stale_while_revalidate_seconds = var.stale_while_revalidate_seconds
    stale_if_error_seconds         = var.stale_if_error_seconds
  }
}

resource "terraform_data" "asset_route" {
  input = local.asset_route
}

resource "terraform_data" "tile_route" {
  input = local.tile_route
}
