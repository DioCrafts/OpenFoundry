use serde_json::{json, Value};

use crate::models::{
	report::ReportDefinition,
	template::{ReportSection, SectionKind},
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

pub fn build_snapshot(report: &ReportDefinition) -> ReportDataSnapshot {
	let seed = (report.name.len() + report.dataset_name.len() + report.template.sections.len()) as i64;
	let highlights = vec![
		ReportHighlight {
			label: "Rows scanned".to_string(),
			value: format!("{}k", 12 + seed),
			delta: format!("+{}%", 3 + (seed % 5)),
		},
		ReportHighlight {
			label: "Freshness".to_string(),
			value: format!("{} min", 6 + (seed % 8)),
			delta: "within SLA".to_string(),
		},
		ReportHighlight {
			label: "Exception rate".to_string(),
			value: format!("{}%", 1 + (seed % 3)),
			delta: "stable".to_string(),
		},
	];

	let sections = report
		.template
		.sections
		.iter()
		.enumerate()
		.map(|(index, section)| section_snapshot(report, section, index as i64))
		.collect();

	ReportDataSnapshot {
		audience_label: report.owner.clone(),
		highlights,
		sections,
	}
}

fn section_snapshot(
	report: &ReportDefinition,
	section: &ReportSection,
	index: i64,
) -> ReportSectionSnapshot {
	let rows = match section.kind {
		SectionKind::Kpi => vec![json!({
			"metric": section.title,
			"value": 84 + index,
			"target": 90,
			"unit": "%"
		})],
		SectionKind::Table => vec![
			json!({"region": "North America", "value": 182000 + index * 2500, "variance": "+4.2%"}),
			json!({"region": "Europe", "value": 149000 + index * 2200, "variance": "+2.1%"}),
			json!({"region": "APAC", "value": 131500 + index * 2000, "variance": "+5.6%"}),
		],
		SectionKind::Chart => vec![
			json!({"bucket": "Mon", "value": 112 + index * 2}),
			json!({"bucket": "Tue", "value": 118 + index * 2}),
			json!({"bucket": "Wed", "value": 124 + index * 2}),
			json!({"bucket": "Thu", "value": 136 + index * 2}),
		],
		SectionKind::Narrative => vec![json!({
			"summary": format!(
				"{} remains on-track for {} with strongest performance in {}.",
				report.name,
				report.dataset_name,
				if index % 2 == 0 { "enterprise accounts" } else { "consumer segments" }
			),
		})],
		SectionKind::Map => vec![
			json!({"location": "Madrid", "lat": 40.4168, "lon": -3.7038, "value": 27 + index}),
			json!({"location": "Paris", "lat": 48.8566, "lon": 2.3522, "value": 21 + index}),
			json!({"location": "Berlin", "lat": 52.52, "lon": 13.405, "value": 18 + index}),
		],
	};

	let summary = match section.kind {
		SectionKind::Kpi => format!("{} is holding above the expected threshold.", section.title),
		SectionKind::Table => format!("{} exposes the ranked regional split for the current cycle.", section.title),
		SectionKind::Chart => format!("{} keeps the last four buckets for trend inspection.", section.title),
		SectionKind::Narrative => format!("{} captures the analyst narrative for the deck.", section.title),
		SectionKind::Map => format!("{} projects geospatial hotspots for executive review.", section.title),
	};

	ReportSectionSnapshot {
		section_id: section.id.clone(),
		title: section.title.clone(),
		kind: section.kind,
		summary,
		rows,
	}
}
