output "edge_profile" {
  value       = terraform_data.edge_cache_profile.output
  description = "Computed global CDN profile for OpenFoundry edge delivery."
}

output "asset_route" {
  value       = terraform_data.asset_route.output
  description = "Static asset cache route with long TTLs and compression enabled."
}

output "tile_route" {
  value       = terraform_data.tile_route.output
  description = "Geospatial tile cache route optimized for short TTL edge reuse."
}