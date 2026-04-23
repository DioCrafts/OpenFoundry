CREATE TABLE IF NOT EXISTS geospatial_layers (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    source_kind TEXT NOT NULL,
    source_dataset TEXT NOT NULL,
    geometry_type TEXT NOT NULL,
    style JSONB NOT NULL DEFAULT '{}'::jsonb,
    features JSONB NOT NULL DEFAULT '[]'::jsonb,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    indexed BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO geospatial_layers (
    id,
    name,
    description,
    source_kind,
    source_dataset,
    geometry_type,
    style,
    features,
    tags,
    indexed,
    created_at,
    updated_at
) VALUES (
    '0196839d-c210-7d23-8e90-7ab001020001',
    'Customer Sites',
    'Point layer representing commercial customer concentration across key metros.',
    'dataset',
    'customer_sites_curated',
    'point',
    jsonb_build_object('color', '#B45309', 'opacity', 0.82, 'radius', 10, 'line_width', 1.5, 'heatmap_intensity', 0.7, 'cluster_color', '#0F766E', 'show_labels', true),
    jsonb_build_array(
        jsonb_build_object('id', 'cust-mad', 'label', 'Madrid Hub', 'geometry', jsonb_build_object('type', 'point', 'coordinates', jsonb_build_object('lat', 40.4168, 'lon', -3.7038)), 'properties', jsonb_build_object('region', 'ES', 'volume', 240)),
        jsonb_build_object('id', 'cust-bcn', 'label', 'Barcelona Hub', 'geometry', jsonb_build_object('type', 'point', 'coordinates', jsonb_build_object('lat', 41.3874, 'lon', 2.1686)), 'properties', jsonb_build_object('region', 'ES', 'volume', 180)),
        jsonb_build_object('id', 'cust-par', 'label', 'Paris Hub', 'geometry', jsonb_build_object('type', 'point', 'coordinates', jsonb_build_object('lat', 48.8566, 'lon', 2.3522)), 'properties', jsonb_build_object('region', 'FR', 'volume', 220)),
        jsonb_build_object('id', 'cust-ber', 'label', 'Berlin Hub', 'geometry', jsonb_build_object('type', 'point', 'coordinates', jsonb_build_object('lat', 52.52, 'lon', 13.405)), 'properties', jsonb_build_object('region', 'DE', 'volume', 205)),
        jsonb_build_object('id', 'cust-lon', 'label', 'London Hub', 'geometry', jsonb_build_object('type', 'point', 'coordinates', jsonb_build_object('lat', 51.5072, 'lon', -0.1276)), 'properties', jsonb_build_object('region', 'UK', 'volume', 260)),
        jsonb_build_object('id', 'cust-nyc', 'label', 'New York Hub', 'geometry', jsonb_build_object('type', 'point', 'coordinates', jsonb_build_object('lat', 40.7128, 'lon', -74.006)), 'properties', jsonb_build_object('region', 'US', 'volume', 310))
    ),
    jsonb_build_array('customers', 'heatmap', 'indexed'),
    true,
    NOW() - interval '30 days',
    NOW() - interval '2 days'
), (
    '0196839d-c210-7d23-8e90-7ab001020002',
    'Delivery Corridors',
    'Line layer for sample route corridors between major customer hubs.',
    'dataset',
    'delivery_corridors',
    'line_string',
    jsonb_build_object('color', '#0F766E', 'opacity', 0.75, 'radius', 7, 'line_width', 3, 'heatmap_intensity', 0.45, 'cluster_color', '#B91C1C', 'show_labels', false),
    jsonb_build_array(
        jsonb_build_object('id', 'corridor-es-fr', 'label', 'Iberia to France', 'geometry', jsonb_build_object('type', 'line_string', 'coordinates', jsonb_build_array(jsonb_build_object('lat', 40.4168, 'lon', -3.7038), jsonb_build_object('lat', 43.3, 'lon', -1.98), jsonb_build_object('lat', 48.8566, 'lon', 2.3522))), 'properties', jsonb_build_object('sla_min', 720)),
        jsonb_build_object('id', 'corridor-fr-de', 'label', 'France to Germany', 'geometry', jsonb_build_object('type', 'line_string', 'coordinates', jsonb_build_array(jsonb_build_object('lat', 48.8566, 'lon', 2.3522), jsonb_build_object('lat', 50.1109, 'lon', 8.6821), jsonb_build_object('lat', 52.52, 'lon', 13.405))), 'properties', jsonb_build_object('sla_min', 810))
    ),
    jsonb_build_array('routes', 'linework'),
    true,
    NOW() - interval '25 days',
    NOW() - interval '3 days'
), (
    '0196839d-c210-7d23-8e90-7ab001020003',
    'Service Zones',
    'Polygon layer for operational service coverage around priority metros.',
    'reference',
    'service_zones_reference',
    'polygon',
    jsonb_build_object('color', '#1D4ED8', 'opacity', 0.36, 'radius', 6, 'line_width', 2, 'heatmap_intensity', 0.3, 'cluster_color', '#B45309', 'show_labels', true),
    jsonb_build_array(
        jsonb_build_object('id', 'zone-madrid', 'label', 'Madrid Service Zone', 'geometry', jsonb_build_object('type', 'polygon', 'coordinates', jsonb_build_array(jsonb_build_object('lat', 40.15, 'lon', -3.95), jsonb_build_object('lat', 40.65, 'lon', -3.95), jsonb_build_object('lat', 40.65, 'lon', -3.25), jsonb_build_object('lat', 40.15, 'lon', -3.25), jsonb_build_object('lat', 40.15, 'lon', -3.95))), 'properties', jsonb_build_object('market', 'ES-CENTRAL')),
        jsonb_build_object('id', 'zone-paris', 'label', 'Paris Service Zone', 'geometry', jsonb_build_object('type', 'polygon', 'coordinates', jsonb_build_array(jsonb_build_object('lat', 48.65, 'lon', 2.05), jsonb_build_object('lat', 49.05, 'lon', 2.05), jsonb_build_object('lat', 49.05, 'lon', 2.65), jsonb_build_object('lat', 48.65, 'lon', 2.65), jsonb_build_object('lat', 48.65, 'lon', 2.05))), 'properties', jsonb_build_object('market', 'FR-IDF')),
        jsonb_build_object('id', 'zone-berlin', 'label', 'Berlin Service Zone', 'geometry', jsonb_build_object('type', 'polygon', 'coordinates', jsonb_build_array(jsonb_build_object('lat', 52.35, 'lon', 13.1), jsonb_build_object('lat', 52.7, 'lon', 13.1), jsonb_build_object('lat', 52.7, 'lon', 13.75), jsonb_build_object('lat', 52.35, 'lon', 13.75), jsonb_build_object('lat', 52.35, 'lon', 13.1))), 'properties', jsonb_build_object('market', 'DE-BE'))
    ),
    jsonb_build_array('coverage', 'polygon'),
    true,
    NOW() - interval '22 days',
    NOW() - interval '4 days'
)
ON CONFLICT (id) DO NOTHING;