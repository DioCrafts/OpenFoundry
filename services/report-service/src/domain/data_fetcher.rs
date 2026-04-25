use std::collections::BTreeMap;

use axum::http::{HeaderMap, header::AUTHORIZATION};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        report::ReportDefinition,
        template::{ReportSection, SectionKind},
    },
};

#[derive(Debug, Clone)]
pub struct ReportHighlight {
    pub label: String,
    pub value: String,
    pub delta: String,
}

#[derive(Debug, Clone)]
pub struct ReportSectionSnapshot {
    pub section_id: String,
    pub title: String,
    pub kind: SectionKind,
    pub summary: String,
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct ReportDataSnapshot {
    pub audience_label: String,
    pub highlights: Vec<ReportHighlight>,
    pub sections: Vec<ReportSectionSnapshot>,
}

#[derive(Debug, Clone)]
struct DatasetSource {
    name: String,
    total_rows: i64,
    rows: Vec<Map<String, Value>>,
    columns: Vec<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug, Clone)]
struct MapPoint {
    label: String,
    lat: f64,
    lon: f64,
    value: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct DatasetSummary {
    id: Uuid,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DatasetListResponse {
    #[serde(default)]
    data: Vec<DatasetSummary>,
}

#[derive(Debug, Clone, Deserialize)]
struct PreviewColumn {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct DatasetPreviewResponse {
    #[serde(default)]
    total_rows: i64,
    #[serde(default)]
    rows: Vec<Value>,
    #[serde(default)]
    columns: Vec<PreviewColumn>,
    #[serde(default)]
    warnings: Vec<String>,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeospatialLayer {
    id: Uuid,
    name: String,
    #[serde(default)]
    features: Vec<GeospatialFeature>,
}

#[derive(Debug, Clone, Deserialize)]
struct LayerListResponse {
    #[serde(default)]
    items: Vec<GeospatialLayer>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeospatialFeature {
    label: String,
    geometry: FeatureGeometry,
    #[serde(default)]
    properties: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct FeatureGeometry {
    #[serde(rename = "type")]
    geometry_type: String,
    coordinates: Value,
}

pub async fn build_snapshot(
    state: &AppState,
    report: &ReportDefinition,
    headers: &HeaderMap,
) -> ReportDataSnapshot {
    let dataset = load_dataset_source(state, report, headers).await;
    let map_points = load_map_points(state, report, headers, dataset.as_ref()).await;

    let highlights = build_highlights(report, dataset.as_ref(), &map_points);
    let sections = report
        .template
        .sections
        .iter()
        .map(|section| materialize_section(report, section, dataset.as_ref(), &map_points))
        .collect();

    ReportDataSnapshot {
        audience_label: report.owner.clone(),
        highlights,
        sections,
    }
}

async fn load_dataset_source(
    state: &AppState,
    report: &ReportDefinition,
    headers: &HeaderMap,
) -> Option<DatasetSource> {
    let dataset_id = report
        .parameters
        .get("dataset_id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .or_else(|| Uuid::parse_str(&report.dataset_name).ok());

    let dataset_id = if let Some(dataset_id) = dataset_id {
        dataset_id
    } else {
        let query = report.dataset_name.trim();
        if query.is_empty() {
            return None;
        }

        let url = format!(
            "{}/api/v1/datasets",
            state.dataset_service_url.trim_end_matches('/')
        );
        let response = authorized_request(&state.http_client, headers, reqwest::Method::GET, &url)
            .query(&[("search", query), ("per_page", "100")])
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let payload = response.json::<DatasetListResponse>().await.ok()?;
        payload
            .data
            .into_iter()
            .find(|dataset| dataset.name == query)
            .map(|dataset| dataset.id)?
    };

    let dataset_url = format!(
        "{}/api/v1/datasets/{dataset_id}",
        state.dataset_service_url.trim_end_matches('/')
    );
    let dataset = authorized_request(
        &state.http_client,
        headers,
        reqwest::Method::GET,
        &dataset_url,
    )
    .send()
    .await
    .ok()?;
    if !dataset.status().is_success() {
        return None;
    }
    let dataset = dataset.json::<DatasetSummary>().await.ok()?;

    let preview_url = format!(
        "{}/api/v1/datasets/{dataset_id}/preview",
        state.dataset_service_url.trim_end_matches('/')
    );
    let preview = authorized_request(
        &state.http_client,
        headers,
        reqwest::Method::GET,
        &preview_url,
    )
    .query(&[("limit", "200")])
    .send()
    .await
    .ok()?;
    if !preview.status().is_success() {
        return None;
    }
    let preview = preview.json::<DatasetPreviewResponse>().await.ok()?;

    Some(DatasetSource {
        name: dataset.name,
        total_rows: preview.total_rows,
        rows: preview
            .rows
            .into_iter()
            .filter_map(|row| row.as_object().cloned())
            .collect(),
        columns: preview
            .columns
            .into_iter()
            .map(|column| column.name)
            .collect(),
        warnings: preview.warnings,
        errors: preview.errors,
    })
}

async fn load_map_points(
    state: &AppState,
    report: &ReportDefinition,
    headers: &HeaderMap,
    dataset: Option<&DatasetSource>,
) -> Vec<MapPoint> {
    if let Some(points) = load_layer_map_points(state, report, headers).await {
        return points;
    }
    dataset.map(extract_dataset_map_points).unwrap_or_default()
}

async fn load_layer_map_points(
    state: &AppState,
    report: &ReportDefinition,
    headers: &HeaderMap,
) -> Option<Vec<MapPoint>> {
    let requested_layer_id = report
        .template
        .sections
        .iter()
        .find_map(|section| section.config.get("layer_id"))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .or_else(|| {
            report
                .parameters
                .get("map_layer_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok())
        });

    let url = format!(
        "{}/api/v1/geospatial/layers",
        state.geospatial_service_url.trim_end_matches('/')
    );
    let response = authorized_request(&state.http_client, headers, reqwest::Method::GET, &url)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload = response.json::<LayerListResponse>().await.ok()?;
    let layers = payload.items;
    let layer = if let Some(layer_id) = requested_layer_id {
        layers.into_iter().find(|layer| layer.id == layer_id)?
    } else {
        let dataset_hint = report.dataset_name.trim();
        layers
            .iter()
            .find(|layer| layer.name.contains(dataset_hint) || layer.name.contains("P6"))
            .cloned()
            .or_else(|| layers.first().cloned())?
    };

    Some(
        layer
            .features
            .into_iter()
            .filter_map(|feature| {
                let (lat, lon) = geometry_centroid(&feature.geometry)?;
                Some(MapPoint {
                    label: feature.label,
                    lat,
                    lon,
                    value: preferred_numeric(
                        &feature.properties,
                        &["value", "count", "amount", "score", "intensity"],
                    ),
                })
            })
            .collect(),
    )
}

fn build_highlights(
    report: &ReportDefinition,
    dataset: Option<&DatasetSource>,
    map_points: &[MapPoint],
) -> Vec<ReportHighlight> {
    let row_count = dataset.map(|dataset| dataset.total_rows).unwrap_or(0);
    let field_count = dataset.map(|dataset| dataset.columns.len()).unwrap_or(0);
    let data_status = dataset
        .map(|dataset| {
            if dataset.errors.is_empty() {
                "live preview".to_string()
            } else {
                format!("{} warning(s)", dataset.errors.len())
            }
        })
        .unwrap_or_else(|| "missing".to_string());

    vec![
        ReportHighlight {
            label: "Rows scanned".to_string(),
            value: row_count.to_string(),
            delta: dataset
                .map(|dataset| format!("dataset {}", dataset.name))
                .unwrap_or_else(|| report.dataset_name.clone()),
        },
        ReportHighlight {
            label: "Fields".to_string(),
            value: field_count.to_string(),
            delta: data_status,
        },
        ReportHighlight {
            label: "Geo points".to_string(),
            value: map_points.len().to_string(),
            delta: if map_points.is_empty() {
                "no map layer".to_string()
            } else {
                "layer-backed".to_string()
            },
        },
    ]
}

fn materialize_section(
    report: &ReportDefinition,
    section: &ReportSection,
    dataset: Option<&DatasetSource>,
    map_points: &[MapPoint],
) -> ReportSectionSnapshot {
    match section.kind {
        SectionKind::Kpi => build_kpi_section(report, section, dataset),
        SectionKind::Table => build_table_section(report, section, dataset),
        SectionKind::Chart => build_chart_section(report, section, dataset),
        SectionKind::Narrative => build_narrative_section(report, section, dataset),
        SectionKind::Map => build_map_section(report, section, dataset, map_points),
    }
}

fn build_kpi_section(
    report: &ReportDefinition,
    section: &ReportSection,
    dataset: Option<&DatasetSource>,
) -> ReportSectionSnapshot {
    let Some(dataset) = dataset else {
        return empty_section(section, "No dataset preview was available for this KPI.");
    };

    let aggregation = section
        .config
        .get("aggregation")
        .and_then(Value::as_str)
        .unwrap_or("count");
    let numeric_field = section
        .config
        .get("field")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| first_numeric_field(&dataset.rows));

    let (value, detail) = match aggregation {
        "avg" => {
            let Some(field) = numeric_field.clone() else {
                return empty_section(section, "No numeric field was found for AVG aggregation.");
            };
            let values = collect_numeric_field(&dataset.rows, &field);
            let avg = average(&values).unwrap_or(0.0);
            (
                json!(round2(avg)),
                format!("AVG of {field} across {} rows", values.len()),
            )
        }
        "sum" => {
            let Some(field) = numeric_field.clone() else {
                return empty_section(section, "No numeric field was found for SUM aggregation.");
            };
            let values = collect_numeric_field(&dataset.rows, &field);
            let sum = values.iter().sum::<f64>();
            (
                json!(round2(sum)),
                format!("SUM of {field} across {} rows", values.len()),
            )
        }
        "max" => {
            let Some(field) = numeric_field.clone() else {
                return empty_section(section, "No numeric field was found for MAX aggregation.");
            };
            let values = collect_numeric_field(&dataset.rows, &field);
            let max = values
                .into_iter()
                .reduce(f64::max)
                .map(round2)
                .unwrap_or(0.0);
            (json!(max), format!("MAX of {field}"))
        }
        "min" => {
            let Some(field) = numeric_field.clone() else {
                return empty_section(section, "No numeric field was found for MIN aggregation.");
            };
            let values = collect_numeric_field(&dataset.rows, &field);
            let min = values
                .into_iter()
                .reduce(f64::min)
                .map(round2)
                .unwrap_or(0.0);
            (json!(min), format!("MIN of {field}"))
        }
        _ => (
            json!(dataset.total_rows),
            "Count of rows in the live dataset preview".to_string(),
        ),
    };

    let summary = format!(
        "{} is derived from dataset '{}' using {} preview rows.",
        section.title,
        report.dataset_name,
        dataset.rows.len()
    );
    ReportSectionSnapshot {
        section_id: section.id.clone(),
        title: section.title.clone(),
        kind: section.kind,
        summary,
        rows: vec![json!({
            "metric": section.title,
            "value": value,
            "detail": detail,
        })],
    }
}

fn build_table_section(
    report: &ReportDefinition,
    section: &ReportSection,
    dataset: Option<&DatasetSource>,
) -> ReportSectionSnapshot {
    let Some(dataset) = dataset else {
        return empty_section(section, "No dataset preview was available for this table.");
    };

    let fields = configured_fields(section)
        .filter(|fields| !fields.is_empty())
        .unwrap_or_else(|| dataset.columns.iter().take(4).cloned().collect());
    let rows = dataset
        .rows
        .iter()
        .take(section_limit(section, 8))
        .map(|row| project_row(row, &fields))
        .collect::<Vec<_>>();

    ReportSectionSnapshot {
        section_id: section.id.clone(),
        title: section.title.clone(),
        kind: section.kind,
        summary: format!(
            "{} shows live rows projected from dataset '{}'.",
            section.title, report.dataset_name
        ),
        rows,
    }
}

fn build_chart_section(
    report: &ReportDefinition,
    section: &ReportSection,
    dataset: Option<&DatasetSource>,
) -> ReportSectionSnapshot {
    let Some(dataset) = dataset else {
        return empty_section(section, "No dataset preview was available for this chart.");
    };

    let group_by = section
        .config
        .get("group_by")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| dataset.columns.first().cloned())
        .unwrap_or_else(|| "bucket".to_string());
    let aggregation = section
        .config
        .get("aggregation")
        .and_then(Value::as_str)
        .unwrap_or("count");
    let metric_field = section
        .config
        .get("metric_field")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or_else(|| first_numeric_field(&dataset.rows));

    let mut buckets = BTreeMap::<String, f64>::new();
    for row in &dataset.rows {
        let key = row
            .get(&group_by)
            .map(display_value)
            .unwrap_or_else(|| "unknown".to_string());
        let entry = buckets.entry(key).or_insert(0.0);
        match aggregation {
            "sum" | "avg" => {
                if let Some(field) = metric_field.as_ref() {
                    *entry += row.get(field).and_then(numeric_value).unwrap_or(0.0);
                } else {
                    *entry += 1.0;
                }
            }
            _ => {
                *entry += 1.0;
            }
        }
    }

    if aggregation == "avg" {
        let mut counts = BTreeMap::<String, usize>::new();
        for row in &dataset.rows {
            let key = row
                .get(&group_by)
                .map(display_value)
                .unwrap_or_else(|| "unknown".to_string());
            *counts.entry(key).or_insert(0) += 1;
        }
        for (key, value) in &mut buckets {
            let count = counts.get(key).copied().unwrap_or(1).max(1) as f64;
            *value = round2(*value / count);
        }
    }

    let rows = buckets
        .into_iter()
        .take(section_limit(section, 8))
        .map(|(bucket, value)| {
            json!({
                "bucket": bucket,
                "value": round2(value),
            })
        })
        .collect();

    ReportSectionSnapshot {
        section_id: section.id.clone(),
        title: section.title.clone(),
        kind: section.kind,
        summary: format!(
            "{} groups the live dataset '{}' by {}.",
            section.title, report.dataset_name, group_by
        ),
        rows,
    }
}

fn build_narrative_section(
    report: &ReportDefinition,
    section: &ReportSection,
    dataset: Option<&DatasetSource>,
) -> ReportSectionSnapshot {
    let summary = if let Some(dataset) = dataset {
        let top_dimension = dataset
            .columns
            .first()
            .and_then(|field| dominant_dimension(&dataset.rows, field).map(|value| (field, value)));
        let warning_count = dataset.warnings.len() + dataset.errors.len();
        match top_dimension {
            Some((field, value)) => format!(
                "{} is grounded on dataset '{}' ({} rows). Dominant {}: {}. {} fetch warning(s).",
                section.title, report.dataset_name, dataset.total_rows, field, value, warning_count
            ),
            None => format!(
                "{} is grounded on dataset '{}' ({} rows). {} fetch warning(s).",
                section.title, report.dataset_name, dataset.total_rows, warning_count
            ),
        }
    } else {
        format!(
            "{} could not load dataset '{}' at generation time.",
            section.title, report.dataset_name
        )
    };

    ReportSectionSnapshot {
        section_id: section.id.clone(),
        title: section.title.clone(),
        kind: section.kind,
        summary: "Narrative summary generated from live report context.".to_string(),
        rows: vec![json!({ "summary": summary })],
    }
}

fn build_map_section(
    report: &ReportDefinition,
    section: &ReportSection,
    dataset: Option<&DatasetSource>,
    map_points: &[MapPoint],
) -> ReportSectionSnapshot {
    if map_points.is_empty() {
        return if let Some(dataset) = dataset {
            empty_section(
                section,
                &format!(
                    "Dataset '{}' has no geospatial coordinates or linked layer for this map.",
                    dataset.name
                ),
            )
        } else {
            empty_section(
                section,
                "No map layer or coordinate-bearing dataset rows were available.",
            )
        };
    }

    let rows = map_points
        .iter()
        .take(section_limit(section, 8))
        .map(|point| {
            json!({
                "location": point.label,
                "lat": point.lat,
                "lon": point.lon,
                "value": point.value.unwrap_or(1.0),
            })
        })
        .collect::<Vec<_>>();

    ReportSectionSnapshot {
        section_id: section.id.clone(),
        title: section.title.clone(),
        kind: section.kind,
        summary: format!(
            "{} projects {} geospatial point(s) tied to report '{}'.",
            section.title,
            map_points.len(),
            report.name
        ),
        rows,
    }
}

fn empty_section(section: &ReportSection, summary: &str) -> ReportSectionSnapshot {
    ReportSectionSnapshot {
        section_id: section.id.clone(),
        title: section.title.clone(),
        kind: section.kind,
        summary: summary.to_string(),
        rows: Vec::new(),
    }
}

fn extract_dataset_map_points(dataset: &DatasetSource) -> Vec<MapPoint> {
    dataset
        .rows
        .iter()
        .filter_map(|row| {
            let lat = row
                .get("lat")
                .or_else(|| row.get("latitude"))
                .and_then(numeric_value)?;
            let lon = row
                .get("lon")
                .or_else(|| row.get("longitude"))
                .and_then(numeric_value)?;
            Some(MapPoint {
                label: row
                    .get("location")
                    .or_else(|| row.get("name"))
                    .or_else(|| row.get("id"))
                    .map(display_value)
                    .unwrap_or_else(|| "point".to_string()),
                lat,
                lon,
                value: preferred_numeric(
                    &Value::Object(row.clone()),
                    &["value", "count", "amount", "score", "revenue"],
                ),
            })
        })
        .collect()
}

fn geometry_centroid(geometry: &FeatureGeometry) -> Option<(f64, f64)> {
    match geometry.geometry_type.as_str() {
        "point" => geometry.coordinates.as_object().and_then(|object| {
            Some((
                numeric_value(object.get("lat")?)?,
                numeric_value(object.get("lon")?)?,
            ))
        }),
        "line_string" | "polygon" => {
            let points = geometry.coordinates.as_array()?;
            let mut lat_sum = 0.0;
            let mut lon_sum = 0.0;
            let mut count = 0.0;
            for point in points {
                let object = point.as_object()?;
                lat_sum += numeric_value(object.get("lat")?)?;
                lon_sum += numeric_value(object.get("lon")?)?;
                count += 1.0;
            }
            if count == 0.0 {
                None
            } else {
                Some((lat_sum / count, lon_sum / count))
            }
        }
        _ => None,
    }
}

fn configured_fields(section: &ReportSection) -> Option<Vec<String>> {
    section
        .config
        .get("fields")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
}

fn section_limit(section: &ReportSection, default_value: usize) -> usize {
    section
        .config
        .get("limit")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(default_value)
}

fn project_row(row: &Map<String, Value>, fields: &[String]) -> Value {
    let projected = fields
        .iter()
        .filter_map(|field| row.get(field).map(|value| (field.clone(), value.clone())))
        .collect::<Map<_, _>>();
    Value::Object(projected)
}

fn first_numeric_field(rows: &[Map<String, Value>]) -> Option<String> {
    rows.iter().find_map(|row| {
        row.iter()
            .find_map(|(key, value)| numeric_value(value).map(|_| key.clone()))
    })
}

fn collect_numeric_field(rows: &[Map<String, Value>], field: &str) -> Vec<f64> {
    rows.iter()
        .filter_map(|row| row.get(field).and_then(numeric_value))
        .collect()
}

fn average(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn dominant_dimension(rows: &[Map<String, Value>], field: &str) -> Option<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for row in rows {
        let key = row
            .get(field)
            .map(display_value)
            .unwrap_or_else(|| "unknown".to_string());
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(key, _)| key)
}

fn preferred_numeric(value: &Value, keys: &[&str]) -> Option<f64> {
    let object = value.as_object()?;
    keys.iter()
        .find_map(|key| object.get(*key).and_then(numeric_value))
}

fn numeric_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse::<f64>().ok(),
        Value::Bool(flag) => Some(if *flag { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn display_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn authorized_request(
    client: &reqwest::Client,
    headers: &HeaderMap,
    method: reqwest::Method,
    url: &str,
) -> reqwest::RequestBuilder {
    let mut request = client.request(method, url);
    if let Some(header_value) = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    {
        request = request.header(AUTHORIZATION.as_str(), header_value);
    }
    request
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, json};

    use crate::models::template::{ReportSection, SectionKind};

    use super::{
        DatasetSource, MapPoint, build_chart_section, build_kpi_section, extract_dataset_map_points,
    };

    fn section(kind: SectionKind, config: serde_json::Value) -> ReportSection {
        ReportSection {
            id: "section-1".to_string(),
            title: "Test section".to_string(),
            kind,
            query: String::new(),
            description: String::new(),
            config,
        }
    }

    fn dataset(rows: &[serde_json::Value]) -> DatasetSource {
        DatasetSource {
            name: "dataset".to_string(),
            total_rows: rows.len() as i64,
            rows: rows
                .iter()
                .filter_map(|row| row.as_object().cloned())
                .collect::<Vec<Map<String, serde_json::Value>>>(),
            columns: vec![
                "region".to_string(),
                "value".to_string(),
                "lat".to_string(),
                "lon".to_string(),
            ],
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    #[test]
    fn computes_live_kpi_sum_from_preview_rows() {
        let source = dataset(&[
            json!({"region": "eu", "value": 12}),
            json!({"region": "us", "value": 8}),
        ]);
        let snapshot = build_kpi_section(
            &crate::models::report::ReportDefinition {
                id: uuid::Uuid::nil(),
                name: "report".to_string(),
                description: String::new(),
                owner: "ops".to_string(),
                generator_kind: crate::models::report::GeneratorKind::Html,
                dataset_name: "dataset".to_string(),
                template: crate::models::template::ReportTemplate {
                    title: String::new(),
                    subtitle: String::new(),
                    theme: String::new(),
                    layout: String::new(),
                    sections: Vec::new(),
                },
                schedule: crate::models::schedule::ReportSchedule::default(),
                recipients: Vec::new(),
                tags: Vec::new(),
                parameters: json!({}),
                active: true,
                last_generated_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            &section(
                SectionKind::Kpi,
                json!({"aggregation": "sum", "field": "value"}),
            ),
            Some(&source),
        );

        assert_eq!(snapshot.rows[0]["value"], json!(20.0));
    }

    #[test]
    fn builds_chart_buckets_from_live_rows() {
        let source = dataset(&[
            json!({"region": "eu", "value": 12}),
            json!({"region": "eu", "value": 8}),
            json!({"region": "us", "value": 7}),
        ]);
        let snapshot = build_chart_section(
            &crate::models::report::ReportDefinition {
                id: uuid::Uuid::nil(),
                name: "report".to_string(),
                description: String::new(),
                owner: "ops".to_string(),
                generator_kind: crate::models::report::GeneratorKind::Html,
                dataset_name: "dataset".to_string(),
                template: crate::models::template::ReportTemplate {
                    title: String::new(),
                    subtitle: String::new(),
                    theme: String::new(),
                    layout: String::new(),
                    sections: Vec::new(),
                },
                schedule: crate::models::schedule::ReportSchedule::default(),
                recipients: Vec::new(),
                tags: Vec::new(),
                parameters: json!({}),
                active: true,
                last_generated_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            &section(
                SectionKind::Chart,
                json!({"group_by": "region", "aggregation": "count"}),
            ),
            Some(&source),
        );

        assert_eq!(snapshot.rows[0]["bucket"], json!("eu"));
        assert_eq!(snapshot.rows[0]["value"], json!(2.0));
    }

    #[test]
    fn extracts_map_points_from_coordinate_rows() {
        let source = dataset(&[
            json!({"name": "Madrid", "lat": 40.4, "lon": -3.7, "value": 9}),
            json!({"name": "Paris", "lat": 48.8, "lon": 2.3, "value": 6}),
        ]);

        let points: Vec<MapPoint> = extract_dataset_map_points(&source);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].label, "Madrid");
        assert_eq!(points[0].value, Some(9.0));
    }
}
