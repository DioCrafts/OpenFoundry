use arrow::util::display::array_value_to_string;
use query_engine::context::QueryContext;
use regex::Regex;
use serde_json::Value;

use crate::models::quality::{DatasetColumnProfile, DatasetQualityRule, DatasetRuleResult};

pub async fn evaluate_rules(
    ctx: &QueryContext,
    rules: &[DatasetQualityRule],
    columns: &[DatasetColumnProfile],
) -> Result<Vec<DatasetRuleResult>, String> {
    let mut results = Vec::new();

    for rule in rules.iter().filter(|rule| rule.enabled) {
        let result = match rule.rule_type.as_str() {
            "null_check" => evaluate_null_check(rule, columns),
            "range" => evaluate_range_check(ctx, rule).await,
            "regex" => evaluate_regex_check(ctx, rule).await,
            "custom_sql" => evaluate_custom_sql(ctx, rule).await,
            other => Ok(DatasetRuleResult {
                rule_id: rule.id,
                name: rule.name.clone(),
                rule_type: other.to_string(),
                severity: rule.severity.clone(),
                passed: false,
                measured_value: None,
                message: "Unknown rule type".to_string(),
            }),
        }?;

        results.push(result);
    }

    Ok(results)
}

fn evaluate_null_check(
    rule: &DatasetQualityRule,
    columns: &[DatasetColumnProfile],
) -> Result<DatasetRuleResult, String> {
    let column_name = config_string(&rule.config, "column")?;
    let max_null_ratio = rule
        .config
        .get("max_null_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);

    let column = columns.iter().find(|column| column.name == column_name);
    let (passed, measured, message) = if let Some(column) = column {
        let passed = column.null_rate <= max_null_ratio;
        (
            passed,
            Some(format!("{:.2}%", column.null_rate * 100.0)),
            format!(
                "Null rate {:.2}% must be <= {:.2}%",
                column.null_rate * 100.0,
                max_null_ratio * 100.0
            ),
        )
    } else {
        (
            false,
            None,
            "Column not found in dataset profile".to_string(),
        )
    };

    Ok(DatasetRuleResult {
        rule_id: rule.id,
        name: rule.name.clone(),
        rule_type: rule.rule_type.clone(),
        severity: rule.severity.clone(),
        passed,
        measured_value: measured,
        message,
    })
}

async fn evaluate_range_check(
    ctx: &QueryContext,
    rule: &DatasetQualityRule,
) -> Result<DatasetRuleResult, String> {
    let column_name = config_string(&rule.config, "column")?;
    let quoted = quote_identifier(&column_name);
    let min_value = rule.config.get("min").and_then(Value::as_f64);
    let max_value = rule.config.get("max").and_then(Value::as_f64);

    if min_value.is_none() && max_value.is_none() {
        return Err("Range rules require at least one boundary".to_string());
    }

    let mut predicates = vec![format!("{quoted} IS NOT NULL")];
    if let Some(min_value) = min_value {
        predicates.push(format!("CAST({quoted} AS DOUBLE) >= {min_value}"));
    }
    if let Some(max_value) = max_value {
        predicates.push(format!("CAST({quoted} AS DOUBLE) <= {max_value}"));
    }

    let passing = fetch_scalar_i64(
        ctx,
        &format!(
            "SELECT COUNT(*) AS value FROM dataset WHERE {}",
            predicates.join(" AND ")
        ),
    )
    .await?;
    let checked = fetch_scalar_i64(
        ctx,
        &format!("SELECT COUNT(*) AS value FROM dataset WHERE {quoted} IS NOT NULL"),
    )
    .await?;
    let failures = checked.saturating_sub(passing);

    Ok(DatasetRuleResult {
        rule_id: rule.id,
        name: rule.name.clone(),
        rule_type: rule.rule_type.clone(),
        severity: rule.severity.clone(),
        passed: failures == 0,
        measured_value: Some(failures.to_string()),
        message: format!("{failures} rows fell outside the allowed range"),
    })
}

async fn evaluate_regex_check(
    ctx: &QueryContext,
    rule: &DatasetQualityRule,
) -> Result<DatasetRuleResult, String> {
    let column_name = config_string(&rule.config, "column")?;
    let pattern = config_string(&rule.config, "pattern")?;
    let allow_nulls = rule
        .config
        .get("allow_nulls")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let regex = Regex::new(&pattern).map_err(|error| error.to_string())?;
    let quoted = quote_identifier(&column_name);
    let filter = if allow_nulls {
        String::new()
    } else {
        format!(" WHERE {quoted} IS NOT NULL")
    };

    let rows = collect_rows(
        ctx,
        &format!(
            "SELECT CAST({quoted} AS VARCHAR) AS value FROM dataset{}",
            filter
        ),
    )
    .await?;

    let mut checked = 0i64;
    let mut failures = 0i64;
    for row in rows {
        let value = row.first().cloned().unwrap_or_default();
        if value == "null" && allow_nulls {
            continue;
        }
        checked += 1;
        if !regex.is_match(&value) {
            failures += 1;
        }
    }

    Ok(DatasetRuleResult {
        rule_id: rule.id,
        name: rule.name.clone(),
        rule_type: rule.rule_type.clone(),
        severity: rule.severity.clone(),
        passed: failures == 0,
        measured_value: Some(format!("{failures}/{checked}")),
        message: format!("{failures} rows did not match the regex"),
    })
}

async fn evaluate_custom_sql(
    ctx: &QueryContext,
    rule: &DatasetQualityRule,
) -> Result<DatasetRuleResult, String> {
    let sql = config_string(&rule.config, "sql")?;
    let operator = rule
        .config
        .get("operator")
        .and_then(Value::as_str)
        .unwrap_or("gte");
    let threshold = rule
        .config
        .get("threshold")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);

    let measured = fetch_scalar_f64(ctx, &sql).await?;
    let passed = compare(measured.unwrap_or(0.0), operator, threshold);

    Ok(DatasetRuleResult {
        rule_id: rule.id,
        name: rule.name.clone(),
        rule_type: rule.rule_type.clone(),
        severity: rule.severity.clone(),
        passed,
        measured_value: measured.map(|value| value.to_string()),
        message: format!(
            "Custom SQL result must satisfy {} {}",
            operator.to_uppercase(),
            threshold
        ),
    })
}

fn compare(measured: f64, operator: &str, threshold: f64) -> bool {
    match operator {
        "eq" => (measured - threshold).abs() < f64::EPSILON,
        "ne" => (measured - threshold).abs() >= f64::EPSILON,
        "gt" => measured > threshold,
        "gte" => measured >= threshold,
        "lt" => measured < threshold,
        "lte" => measured <= threshold,
        _ => measured >= threshold,
    }
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn config_string(config: &Value, key: &str) -> Result<String, String> {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Missing rule config field: {key}"))
}

async fn collect_rows(ctx: &QueryContext, sql: &str) -> Result<Vec<Vec<String>>, String> {
    let batches = ctx
        .execute_sql(sql)
        .await
        .map_err(|error| error.to_string())?;
    let mut rows = Vec::new();

    for batch in batches {
        for row_index in 0..batch.num_rows() {
            let mut row = Vec::new();
            for column_index in 0..batch.num_columns() {
                let value = array_value_to_string(batch.column(column_index), row_index)
                    .unwrap_or_else(|_| "null".to_string());
                row.push(value);
            }
            rows.push(row);
        }
    }

    Ok(rows)
}

async fn fetch_scalar_i64(ctx: &QueryContext, sql: &str) -> Result<i64, String> {
    let rows = collect_rows(ctx, sql).await?;
    Ok(rows
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0))
}

async fn fetch_scalar_f64(ctx: &QueryContext, sql: &str) -> Result<Option<f64>, String> {
    let rows = collect_rows(ctx, sql).await?;
    Ok(rows
        .first()
        .and_then(|row| row.first())
        .and_then(|value| value.parse::<f64>().ok()))
}
