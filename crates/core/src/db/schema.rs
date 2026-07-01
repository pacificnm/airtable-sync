//! Database schema introspection for `db schema`.

use std::path::{Path, PathBuf};

use nest_cli::CliGlobals;
use nest_core::AppContext;
use nest_data::DataError;
use nest_error::{NestError, NestResult};
use rusqlite::Connection;
use serde::Serialize;

use crate::config::{ensure_valid_config, print_warning, resolve_config_path};

use super::common::{absolute_path, open_database};

const MIGRATIONS_TABLE: &str = "_nest_migrations";

/// Full schema view for JSON and human output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DbSchemaView {
    /// Absolute path to the SQLite database file.
    pub database_path: PathBuf,
    /// Configured schema SQL file path (context only).
    pub schema_file: PathBuf,
    /// Applied migration ids, oldest first.
    pub migrations: Vec<String>,
    /// User-facing tables and columns.
    pub tables: Vec<TableSchemaView>,
}

/// One table and its columns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableSchemaView {
    /// Table name.
    pub name: String,
    /// Columns in table definition order.
    pub columns: Vec<ColumnSchemaView>,
}

/// One column from `PRAGMA table_info`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ColumnSchemaView {
    /// Column name.
    pub name: String,
    /// Declared type.
    pub type_name: String,
    /// Whether the column is `NOT NULL`.
    pub not_null: bool,
    /// Default value expression, if any.
    pub default_value: Option<String>,
    /// Whether the column is part of the primary key.
    pub primary_key: bool,
}

/// Displays schema information for the configured SQLite database.
pub fn schema(ctx: &AppContext) -> NestResult<()> {
    let validated = ensure_valid_config(ctx)?;

    let globals = ctx.service::<CliGlobals>().ok();
    let quiet = globals.as_ref().is_some_and(|globals| globals.quiet);
    let json = globals.as_ref().is_some_and(|globals| globals.json);

    if !quiet {
        for warning in validated.warnings {
            print_warning(&warning);
        }
    }

    let database_path = resolve_config_path(&validated.config, &validated.app.database.database_path);
    let schema_path = resolve_config_path(&validated.config, &validated.app.database.schema);

    if !database_path.is_file() {
        return Err(NestError::data(format!(
            "database file not found: {}",
            database_path.display()
        ))
        .with_help(
            "Run `db init` (first time) or `db reset --yes` to create the database.",
        ));
    }

    let view = introspect_database(&database_path, &schema_path)?;

    if json {
        let payload = serde_json::to_string_pretty(&view).map_err(|error| {
            NestError::data(format!("failed to serialize db schema: {error}"))
        })?;
        println!("{payload}");
    } else if !quiet {
        println!("{}", format_schema_human(&view));
    }

    Ok(())
}

/// Introspects the live SQLite database and returns structured schema metadata.
pub fn introspect_database(
    database_path: &Path,
    schema_path: &Path,
) -> NestResult<DbSchemaView> {
    let conn = open_database(database_path)?;
    let migrations = conn
        .with_connection(|db| load_migrations(db))
        .map_err(NestError::from)?;
    let tables = conn
        .with_connection(|db| load_tables(db))
        .map_err(NestError::from)?;

    Ok(DbSchemaView {
        database_path: absolute_path(database_path),
        schema_file: absolute_path(schema_path),
        migrations,
        tables,
    })
}

/// Formats the schema view as human-readable sectioned text.
pub fn format_schema_human(view: &DbSchemaView) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "Database: {}\nSchema file: {}\n",
        view.database_path.display(),
        view.schema_file.display()
    ));

    output.push_str("\nMigrations:\n");
    if view.migrations.is_empty() {
        output.push_str("  (none)\n");
    } else {
        for migration in &view.migrations {
            output.push_str(&format!("  {migration}\n"));
        }
    }

    for table in &view.tables {
        output.push_str(&format!("\nTable: {}\n", table.name));
        for column in &table.columns {
            output.push_str(&format!("  {}\n", format_column_human(column)));
        }
    }

    output
}

fn format_column_human(column: &ColumnSchemaView) -> String {
    let mut parts = vec![column.name.clone(), column.type_name.clone()];
    if column.primary_key {
        parts.push("PRIMARY KEY".to_string());
    }
    if column.not_null {
        parts.push("NOT NULL".to_string());
    }
    if let Some(default) = &column.default_value {
        parts.push(format!("DEFAULT {default}"));
    }
    parts.join(" ")
}

fn load_migrations(db: &Connection) -> Result<Vec<String>, DataError> {
    if !table_exists(db, MIGRATIONS_TABLE)? {
        return Ok(Vec::new());
    }

    let mut stmt = map_query_error(db.prepare(&format!(
        "SELECT id FROM {MIGRATIONS_TABLE} ORDER BY rowid ASC"
    )))?;
    let rows = map_query_error(stmt.query_map([], |row| row.get::<_, String>(0)))?;
    let mut migrations = Vec::new();
    for row in rows {
        migrations.push(map_query_error(row)?);
    }
    Ok(migrations)
}

fn load_tables(db: &Connection) -> Result<Vec<TableSchemaView>, DataError> {
    let mut stmt = map_query_error(db.prepare(
        "SELECT name FROM sqlite_master \
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name != ?1 \
         ORDER BY name ASC",
    ))?;
    let rows = map_query_error(stmt.query_map([MIGRATIONS_TABLE], |row| {
        row.get::<_, String>(0)
    }))?;

    let mut tables = Vec::new();
    for row in rows {
        let name = map_query_error(row)?;
        let columns = load_columns(db, &name)?;
        tables.push(TableSchemaView { name, columns });
    }
    Ok(tables)
}

fn load_columns(db: &Connection, table: &str) -> Result<Vec<ColumnSchemaView>, DataError> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = map_query_error(db.prepare(&sql))?;
    let rows = map_query_error(stmt.query_map([], |row| {
        Ok(ColumnSchemaView {
            name: row.get(1)?,
            type_name: row.get(2)?,
            not_null: row.get::<_, i64>(3)? != 0,
            default_value: row.get(4)?,
            primary_key: row.get::<_, i64>(5)? != 0,
        })
    }))?;

    let mut columns = Vec::new();
    for row in rows {
        columns.push(map_query_error(row)?);
    }
    Ok(columns)
}

fn table_exists(db: &Connection, name: &str) -> Result<bool, DataError> {
    let count: i64 = map_query_error(db.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get(0),
    ))?;
    Ok(count > 0)
}

fn map_query_error<T>(result: rusqlite::Result<T>) -> Result<T, DataError> {
    result.map_err(|error| DataError::query(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_output_includes_database_tables_and_columns() {
        let view = DbSchemaView {
            database_path: PathBuf::from("/tmp/app.db"),
            schema_file: PathBuf::from("/tmp/schema.sql"),
            migrations: vec!["001_initial_schema".to_string()],
            tables: vec![TableSchemaView {
                name: "airtable_tables".to_string(),
                columns: vec![
                    ColumnSchemaView {
                        name: "id".to_string(),
                        type_name: "INTEGER".to_string(),
                        not_null: true,
                        default_value: None,
                        primary_key: true,
                    },
                    ColumnSchemaView {
                        name: "table_id".to_string(),
                        type_name: "TEXT".to_string(),
                        not_null: true,
                        default_value: None,
                        primary_key: false,
                    },
                ],
            }],
        };

        let output = format_schema_human(&view);
        assert!(output.contains("Database: /tmp/app.db"));
        assert!(output.contains("001_initial_schema"));
        assert!(output.contains("Table: airtable_tables"));
        assert!(output.contains("table_id TEXT NOT NULL"));
    }
}
