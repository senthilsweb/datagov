//! inspect.rs — `datagov inspect`: thin adapter, no business logic.
//!
//! 1. Resolves the input `Format` (via `datagov_data::format::detect`),
//!    constructs a `datagov_data::Source` (a real path, or stdin), and
//!    calls the matching `datagov-data` reader.
//! 2. Wraps the resulting `DatasetSection` into the Bolt 1
//!    `ReportBuilder`: `input.uri` is the path as given (`"-"` for
//!    stdin), `input.format` is the resolved format, and
//!    `input.content_hash` is the SHA-256 of the file — omitted
//!    entirely for stdin, since there is nothing stable to hash.
//! 3. Human rendering (`comfy-table`): format, human-readable file size,
//!    row/column counts, a schema table (name/type/nullable), a
//!    Parquet row-group/compression summary line when applicable, and a
//!    small already-masked sample-rows table.
//! 4. `--output json`: the full report envelope.

use std::path::PathBuf;

use comfy_table::Table;
use datagov_core::DatagovError;
use datagov_core::report::{DataType, Input, ReportBuilder, Sections};

use crate::cli::OutputFormat;

pub fn run(path: &str, type_flag: Option<&str>, output: OutputFormat) -> Result<(), DatagovError> {
    let format = datagov_data::format::detect(path, type_flag)?;

    let (source, content_hash) = if path == "-" {
        let source = datagov_data::Source::from_stdin(Box::new(std::io::stdin()), format);
        (source, None)
    } else {
        let path_buf = PathBuf::from(path);
        let content_hash =
            datagov_core::report::content_hash_sha256(&path_buf).map_err(|source| {
                DatagovError::input_not_found(
                    format!("input not found: {path} ({source})"),
                    format!("check that {path} exists and is readable"),
                )
            })?;
        let source = datagov_data::Source::from_path(path_buf, format);
        (source, Some(content_hash))
    };

    let reader = datagov_data::reader_for(format);
    let dataset = reader.inspect(&source)?;

    let input = Input {
        uri: source.uri(),
        format: format.as_str().to_string(),
        content_hash,
    };

    match output {
        OutputFormat::Table => render_table(&dataset),
        OutputFormat::Json => {
            let report = ReportBuilder::new()
                .input(input)
                .sections(Sections {
                    dataset: Some(dataset),
                    ..Default::default()
                })
                .build();
            let json = serde_json::to_string(&report)
                .map_err(|e| DatagovError::internal(format!("failed to render report: {e}")))?;
            println!("{json}");
        }
    }

    Ok(())
}

fn render_table(dataset: &datagov_core::report::DatasetSection) {
    println!("format:       {}", dataset.format);
    println!("file size:    {}", humanize_bytes(dataset.file_size_bytes));
    println!("row count:    {}", dataset.row_count);
    println!("column count: {}", dataset.column_count);

    let mut schema_table = Table::new();
    schema_table.set_header(vec!["name", "type", "nullable"]);
    for column in &dataset.schema {
        schema_table.add_row(vec![
            column.name.clone(),
            data_type_label(column.data_type).to_string(),
            column.nullable.to_string(),
        ]);
    }
    println!("{schema_table}");

    if let Some(parquet) = &dataset.parquet {
        println!(
            "row groups: {}, compression: {}",
            parquet.row_groups.len(),
            parquet.compression_codecs.join(", ")
        );
    }

    if !dataset.sample_rows.is_empty() {
        let mut sample_table = Table::new();
        let columns: Vec<String> = dataset.schema.iter().map(|c| c.name.clone()).collect();
        sample_table.set_header(columns.clone());
        for row in &dataset.sample_rows {
            let cells: Vec<String> = columns
                .iter()
                .map(|name| {
                    row.get(name)
                        .map(render_json_cell)
                        .unwrap_or_else(|| "null".to_string())
                })
                .collect();
            sample_table.add_row(cells);
        }
        println!("{sample_table}");
    }
}

fn render_json_cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn data_type_label(data_type: DataType) -> &'static str {
    match data_type {
        DataType::Boolean => "boolean",
        DataType::Integer => "integer",
        DataType::Float => "float",
        DataType::String => "string",
        DataType::Object => "object",
        DataType::Array => "array",
        DataType::Null => "null",
        DataType::Mixed => "mixed",
    }
}

/// Render a byte count as a human-readable size (e.g. `"482.9 KiB"`),
/// using binary (1024) units.
fn humanize_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit_index = 0;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        format!("{bytes} {}", UNITS[0])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::humanize_bytes;

    #[test]
    fn humanizes_bytes_across_units() {
        assert_eq!(humanize_bytes(0), "0 B");
        assert_eq!(humanize_bytes(512), "512 B");
        assert_eq!(humanize_bytes(1536), "1.5 KiB");
        assert_eq!(humanize_bytes(494490), "482.9 KiB");
    }
}
