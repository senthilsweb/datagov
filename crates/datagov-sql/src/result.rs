//! result.rs — `SqlParseResult` and its nested types, and the AST ->
//! best-effort-projection logic that builds them from a parsed
//! `sqlglot_rust::Statement`.
//!
//! 1. `SqlParseResult` mirrors the Bolt 4 brief's shape exactly:
//!    `dialect`, `statement_type`, `tables`, `columns`, `joins`,
//!    `filters`, `group_by`, `order_by`, `ctes`, `ast`.
//! 2. **On `ast`**: this is `sqlglot_rust::Statement` serialized directly
//!    via its own `Serialize` derive (`serde_json::to_value`) — not a
//!    hand-rolled universal AST schema. The PRD's "normalized AST
//!    representation" requirement is satisfied by exposing the crate's
//!    real, complete parse tree (every statement/expression variant it
//!    supports), not a lossy abstraction over it. Shape is whatever
//!    `sqlglot_rust::Statement`'s derive produces (an externally-tagged
//!    enum, e.g. `{"Select": {...}}`) — see the crate's own
//!    "Serialization (JSON Round-Tripping)" guide section.
//! 3. **`tables`** comes from `sqlglot_rust::ast::find_tables`, which
//!    walks the *entire* statement (FROM, JOINs, subqueries, CTE
//!    bodies) — so it can list more entries than `joins` alone. Each
//!    entry's `name` is the dotted `catalog.schema.name` (only the parts
//!    the parser populated), `alias` is the parser's table alias.
//! 4. **`columns`/`joins`/`filters`/`group_by`/`order_by`/`ctes`** are
//!    populated only when the statement is a `Statement::Select` — the
//!    brief's fields are inherently SELECT-shaped (an INSERT/UPDATE/
//!    DELETE/CREATE TABLE has no "selected columns" or "GROUP BY" of its
//!    own); non-SELECT statements get empty vectors for these fields
//!    while `tables`/`statement_type`/`ast` are still fully populated.
//! 5. Expression rendering (`columns`, `filters`, `group_by`,
//!    `order_by`, join `on`) uses `Expr::sql()` — the crate's own
//!    "quick SQL output (ANSI dialect)" renderer (see its API
//!    reference), not a dialect-parameterized `generate()` call. This
//!    matches the brief's "rendered filter expressions as text,
//!    best-effort" framing: it's readable, deterministic text for a
//!    report, not necessarily valid syntax in every source dialect
//!    (e.g. a Snowflake `ILIKE` renders the same way regardless of
//!    dialect). `filters` splits the `WHERE` clause on top-level `AND`
//!    conjuncts into one string per conjunct (a `HAVING` clause, if
//!    present, is not surfaced here — out of scope for this bolt's
//!    `filters` field, which the spec ties to `WHERE`).
//! 6. `TableRef`/`JoinRef` are the brief's flat, report-friendly shapes
//!    (`{name, alias}` / `{kind, table, on}`) — table sources that are
//!    not a plain table (a subquery, table function, `LATERAL`,
//!    `UNNEST`, `PIVOT`/`UNPIVOT`) render `table` as a best-effort text
//!    description (see `describe_table_source`) rather than a bare
//!    name, since `JoinRef::table` is a `String`, not a `TableSource`.

use serde::Serialize;
use sqlglot_rust::ast::{
    Cte, Expr, JoinClause, JoinType, OrderByItem, SelectItem, SelectStatement, TableSource,
};
use sqlglot_rust::{Statement, ast::find_tables};

/// The `datagov sql parse` result payload, serialized into
/// `extensions.sql_parse` by the CLI.
#[derive(Debug, Clone, Serialize)]
pub struct SqlParseResult {
    pub dialect: String,
    /// `"select"` | `"insert"` | `"update"` | `"delete"` |
    /// `"create_table"` | `"other"` (every other `Statement` variant —
    /// `MERGE`, `DROP TABLE`, set operations, `ALTER TABLE`, views,
    /// `TRUNCATE`, transactions, `EXPLAIN`, `USE`, bare expressions).
    pub statement_type: String,
    pub tables: Vec<TableRef>,
    /// Top-level selected columns, best-effort; empty for non-`SELECT`
    /// statements.
    pub columns: Vec<String>,
    pub joins: Vec<JoinRef>,
    /// Each `WHERE`-clause top-level `AND` conjunct, rendered as text;
    /// empty when there is no `WHERE` clause or the statement is not a
    /// `SELECT`.
    pub filters: Vec<String>,
    pub group_by: Vec<String>,
    pub order_by: Vec<String>,
    /// CTE names (the `WITH` clause), in declaration order.
    pub ctes: Vec<String>,
    /// The full `sqlglot_rust::Statement` AST, serialized as-is. See the
    /// module header for why this is the crate's native tree rather
    /// than a hand-rolled schema.
    pub ast: serde_json::Value,
}

/// A referenced table: name plus alias, as the brief specifies. `name`
/// is the dotted `catalog.schema.name` for whichever parts the parser
/// populated (often just `name`).
#[derive(Debug, Clone, Serialize)]
pub struct TableRef {
    pub name: String,
    pub alias: Option<String>,
}

/// One `JOIN` clause: its kind, the joined table (best-effort text for
/// non-plain-table sources — see the module header), and its `ON`
/// predicate rendered as text (`None` for a `USING`-only or natural
/// join with no explicit predicate).
#[derive(Debug, Clone, Serialize)]
pub struct JoinRef {
    pub kind: String,
    pub table: String,
    pub on: Option<String>,
}

/// Build the full `SqlParseResult` for a parsed statement under
/// `dialect_label` (the canonical dialect name already resolved by
/// `crate::dialect::resolve`).
pub fn build(
    stmt: &Statement,
    dialect_label: &str,
) -> Result<SqlParseResult, datagov_core::DatagovError> {
    let ast = serde_json::to_value(stmt).map_err(|e| {
        datagov_core::DatagovError::internal(format!("failed to serialize parsed SQL AST: {e}"))
    })?;

    let tables: Vec<TableRef> = find_tables(stmt)
        .into_iter()
        .map(|t| TableRef {
            name: qualified_table_name(t),
            alias: t.alias.clone(),
        })
        .collect();

    let select = match stmt {
        Statement::Select(select) => Some(select),
        _ => None,
    };

    Ok(SqlParseResult {
        dialect: dialect_label.to_string(),
        statement_type: statement_type_label(stmt).to_string(),
        tables,
        columns: select.map(select_columns).unwrap_or_default(),
        joins: select
            .map(|s| s.joins.iter().map(describe_join).collect())
            .unwrap_or_default(),
        filters: select
            .and_then(|s| s.where_clause.as_ref())
            .map(|expr| {
                split_and_conjuncts(expr)
                    .into_iter()
                    .map(Expr::sql)
                    .collect()
            })
            .unwrap_or_default(),
        group_by: select
            .map(|s| s.group_by.iter().map(Expr::sql).collect())
            .unwrap_or_default(),
        order_by: select
            .map(|s| s.order_by.iter().map(describe_order_by_item).collect())
            .unwrap_or_default(),
        ctes: select
            .map(|s| s.ctes.iter().map(describe_cte).collect())
            .unwrap_or_default(),
        ast,
    })
}

fn statement_type_label(stmt: &Statement) -> &'static str {
    match stmt {
        Statement::Select(_) => "select",
        Statement::Insert(_) => "insert",
        Statement::Update(_) => "update",
        Statement::Delete(_) => "delete",
        Statement::CreateTable(_) => "create_table",
        _ => "other",
    }
}

fn qualified_table_name(t: &sqlglot_rust::ast::TableRef) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(3);
    if let Some(catalog) = &t.catalog {
        parts.push(catalog);
    }
    if let Some(schema) = &t.schema {
        parts.push(schema);
    }
    parts.push(&t.name);
    parts.join(".")
}

fn select_columns(select: &SelectStatement) -> Vec<String> {
    select
        .columns
        .iter()
        .map(|item| match item {
            SelectItem::Wildcard => "*".to_string(),
            SelectItem::QualifiedWildcard { table } => format!("{table}.*"),
            SelectItem::Expr { expr, alias, .. } => match alias {
                Some(a) => format!("{} AS {a}", expr.sql()),
                None => expr.sql(),
            },
        })
        .collect()
}

fn join_type_label(join_type: &JoinType) -> &'static str {
    match join_type {
        JoinType::Inner => "inner",
        JoinType::Left => "left",
        JoinType::Right => "right",
        JoinType::Full => "full",
        JoinType::Cross => "cross",
        JoinType::Natural => "natural",
        JoinType::Lateral => "lateral",
    }
}

fn describe_join(join: &JoinClause) -> JoinRef {
    JoinRef {
        kind: join_type_label(&join.join_type).to_string(),
        table: describe_table_source(&join.table),
        on: join.on.as_ref().map(Expr::sql),
    }
}

/// Best-effort text description of a `TableSource` for `JoinRef::table`
/// (a `String`, not a `TableSource`). Plain tables render as their
/// qualified name (plus alias); everything else (subqueries, table
/// functions, `LATERAL`, `UNNEST`, `PIVOT`/`UNPIVOT`) renders a short,
/// readable label rather than attempting full SQL reconstruction.
fn describe_table_source(source: &TableSource) -> String {
    match source {
        TableSource::Table(t) => {
            let name = qualified_table_name(t);
            match &t.alias {
                Some(alias) => format!("{name} AS {alias}"),
                None => name,
            }
        }
        TableSource::Subquery { alias, .. } => match alias {
            Some(alias) => format!("(subquery) AS {alias}"),
            None => "(subquery)".to_string(),
        },
        TableSource::TableFunction { name, alias, .. } => match alias {
            Some(alias) => format!("{name}(...) AS {alias}"),
            None => format!("{name}(...)"),
        },
        TableSource::Lateral { source } => format!("LATERAL {}", describe_table_source(source)),
        TableSource::Unnest { alias, .. } => match alias {
            Some(alias) => format!("UNNEST(...) AS {alias}"),
            None => "UNNEST(...)".to_string(),
        },
        TableSource::Pivot { source, alias, .. } => {
            let base = describe_table_source(source);
            match alias {
                Some(alias) => format!("{base} PIVOT(...) AS {alias}"),
                None => format!("{base} PIVOT(...)"),
            }
        }
        TableSource::Unpivot { source, alias, .. } => {
            let base = describe_table_source(source);
            match alias {
                Some(alias) => format!("{base} UNPIVOT(...) AS {alias}"),
                None => format!("{base} UNPIVOT(...)"),
            }
        }
    }
}

/// Split a `WHERE`-clause expression on top-level `AND` into its
/// conjuncts (recursively — `a AND b AND c` yields three entries), so
/// `filters` reports each predicate independently rather than one
/// giant string. Anything else (an `OR`, a single predicate, ...) is
/// returned as one entry.
fn split_and_conjuncts(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::BinaryOp {
            left,
            op: sqlglot_rust::ast::BinaryOperator::And,
            right,
        } => {
            let mut conjuncts = split_and_conjuncts(left);
            conjuncts.extend(split_and_conjuncts(right));
            conjuncts
        }
        other => vec![other],
    }
}

fn describe_order_by_item(item: &OrderByItem) -> String {
    let direction = if item.ascending { "ASC" } else { "DESC" };
    match item.nulls_first {
        Some(true) => format!("{} {direction} NULLS FIRST", item.expr.sql()),
        Some(false) => format!("{} {direction} NULLS LAST", item.expr.sql()),
        None => format!("{} {direction}", item.expr.sql()),
    }
}

fn describe_cte(cte: &Cte) -> String {
    cte.name.clone()
}
