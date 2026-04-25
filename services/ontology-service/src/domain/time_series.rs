use serde_json::{Value, json};

use crate::models::quiver::QuiverVisualFunctionDraft;

pub fn normalize_chart_kind(chart_kind: &str) -> Result<&'static str, String> {
    match chart_kind {
        "line" => Ok("line"),
        "area" => Ok("area"),
        "bar" => Ok("bar"),
        "point" => Ok("point"),
        _ => Err(format!(
            "chart_kind must be one of: line, area, bar, point (received {chart_kind})"
        )),
    }
}

pub fn build_quiver_vega_spec(draft: &QuiverVisualFunctionDraft) -> Result<Value, String> {
    let chart_kind = normalize_chart_kind(&draft.chart_kind)?;
    let selected_group = draft.selected_group.clone().unwrap_or_default();
    let join_mode = if draft.secondary_type_id.is_some() {
        "joined_object_sets"
    } else {
        "single_object_set"
    };
    let time_series_mark = if chart_kind == "area" {
        json!({
            "type": "area",
            "line": true,
            "point": true,
            "interpolate": "monotone",
            "opacity": 0.22
        })
    } else {
        json!({
            "type": chart_kind,
            "point": chart_kind != "bar",
            "interpolate": "monotone"
        })
    };

    Ok(json!({
        "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
        "description": if draft.description.trim().is_empty() {
            format!("Quiver visual function '{}' generated from ontology analytics.", draft.name)
        } else {
            draft.description.clone()
        },
        "title": {
            "text": draft.name,
            "subtitle": format!(
                "{} • metric {} • group {} • join mode {}",
                draft.date_field,
                draft.metric_field,
                draft.group_field,
                join_mode
            ),
            "anchor": "start"
        },
        "spacing": 20,
        "background": "#ffffff",
        "datasets": {
            "timeSeries": [],
            "grouped": []
        },
        "params": [
            {
                "name": "selectedGroup",
                "value": selected_group
            }
        ],
        "vconcat": [
            {
                "data": { "name": "timeSeries" },
                "mark": time_series_mark,
                "encoding": {
                    "x": {
                        "field": "date",
                        "type": "temporal",
                        "title": draft.date_field
                    },
                    "y": {
                        "field": "value",
                        "type": "quantitative",
                        "title": draft.metric_field
                    },
                    "tooltip": [
                        { "field": "date", "type": "temporal", "title": draft.date_field },
                        { "field": "value", "type": "quantitative", "title": draft.metric_field },
                        { "field": "count", "type": "quantitative", "title": "count" }
                    ]
                }
            },
            {
                "data": { "name": "grouped" },
                "mark": {
                    "type": "bar",
                    "cornerRadiusTopLeft": 6,
                    "cornerRadiusTopRight": 6
                },
                "encoding": {
                    "x": {
                        "field": "group",
                        "type": "nominal",
                        "sort": "-y",
                        "title": draft.group_field
                    },
                    "y": {
                        "field": "value",
                        "type": "quantitative",
                        "title": draft.metric_field
                    },
                    "color": {
                        "field": "group",
                        "type": "nominal",
                        "legend": null
                    },
                    "tooltip": [
                        { "field": "group", "type": "nominal", "title": draft.group_field },
                        { "field": "value", "type": "quantitative", "title": draft.metric_field },
                        { "field": "count", "type": "quantitative", "title": "count" }
                    ]
                }
            }
        ],
        "config": {
            "view": { "stroke": "#dbe4ea", "cornerRadius": 18 },
            "axis": {
                "labelColor": "#334155",
                "titleColor": "#0f172a",
                "gridColor": "#e2e8f0",
                "tickColor": "#cbd5e1"
            },
            "header": {
                "titleColor": "#0f172a",
                "labelColor": "#475569"
            }
        },
        "usermeta": {
            "quiver": {
                "primary_type_id": draft.primary_type_id,
                "secondary_type_id": draft.secondary_type_id,
                "join_field": draft.join_field,
                "secondary_join_field": draft.secondary_join_field,
                "date_field": draft.date_field,
                "metric_field": draft.metric_field,
                "group_field": draft.group_field,
                "selected_group": draft.selected_group,
                "chart_kind": chart_kind,
                "shared": draft.shared
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn sample_draft() -> QuiverVisualFunctionDraft {
        QuiverVisualFunctionDraft {
            name: "Pipeline Throughput".to_string(),
            description: "Tracks daily throughput by team.".to_string(),
            primary_type_id: Uuid::nil(),
            secondary_type_id: Some(Uuid::now_v7()),
            join_field: "order_id".to_string(),
            secondary_join_field: "order_id".to_string(),
            date_field: "event_date".to_string(),
            metric_field: "throughput".to_string(),
            group_field: "team".to_string(),
            selected_group: Some("EMEA".to_string()),
            chart_kind: "area".to_string(),
            shared: true,
        }
    }

    #[test]
    fn builds_a_vega_lite_template_for_quiver() {
        let spec = build_quiver_vega_spec(&sample_draft()).expect("spec should build");

        assert_eq!(
            spec.get("$schema").and_then(Value::as_str),
            Some("https://vega.github.io/schema/vega-lite/v5.json")
        );
        assert_eq!(
            spec.pointer("/datasets/timeSeries")
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(0)
        );
        assert_eq!(
            spec.pointer("/vconcat/0/mark/type").and_then(Value::as_str),
            Some("area")
        );
        assert_eq!(
            spec.pointer("/usermeta/quiver/chart_kind")
                .and_then(Value::as_str),
            Some("area")
        );
        assert_eq!(
            spec.pointer("/params/0/value").and_then(Value::as_str),
            Some("EMEA")
        );
    }

    #[test]
    fn rejects_unknown_chart_kind() {
        let mut draft = sample_draft();
        draft.chart_kind = "heatmap".to_string();

        let error = build_quiver_vega_spec(&draft).expect_err("chart kind should fail");
        assert!(error.contains("chart_kind"));
    }
}
