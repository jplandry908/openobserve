// Copyright 2025 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use async_trait::async_trait;
use config::{meta::stream::StreamType, utils::json};
use datafusion::arrow::datatypes::Schema;

use crate::{
    db::{
        IndexStatement,
        sqlite::{CLIENT_RW, create_index},
    },
    errors::{Error, Result},
};

pub struct SqliteSchemaHistory {}

impl SqliteSchemaHistory {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SqliteSchemaHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::SchemaHistory for SqliteSchemaHistory {
    async fn create_table(&self) -> Result<()> {
        create_table().await
    }

    async fn create_table_index(&self) -> Result<()> {
        create_table_index().await
    }

    async fn create(
        &self,
        org_id: &str,
        stream_type: StreamType,
        stream_name: &str,
        start_dt: i64,
        schema: Schema,
    ) -> Result<()> {
        let value = json::to_string(&schema)?;
        let client = CLIENT_RW.clone();
        let client = client.lock().await;
        match sqlx::query(
            r#"
INSERT INTO schema_history (org, stream_type, stream_name, start_dt, value)
    VALUES ($1, $2, $3, $4, $5);
        "#,
        )
        .bind(org_id)
        .bind(stream_type.as_str())
        .bind(stream_name)
        .bind(start_dt)
        .bind(value)
        .execute(&*client)
        .await
        {
            Err(sqlx::Error::Database(e)) => {
                if e.is_unique_violation() {
                    Ok(())
                } else {
                    Err(Error::Message(e.to_string()))
                }
            }
            Err(e) => Err(e.into()),
            Ok(_) => Ok(()),
        }
    }
}

pub async fn create_table() -> Result<()> {
    let client = CLIENT_RW.clone();
    let client = client.lock().await;
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS schema_history
(
    id           INTEGER not null primary key autoincrement,
    org          VARCHAR not null,
    stream_type  VARCHAR not null,
    stream_name  VARCHAR not null,
    start_dt     INTEGER not null,
    value        TEXT not null
);
        "#,
    )
    .execute(&*client)
    .await?;

    Ok(())
}

pub async fn create_table_index() -> Result<()> {
    create_index(IndexStatement::new(
        "schema_history_org_idx",
        "schema_history",
        false,
        &["org"],
    ))
    .await?;
    create_index(IndexStatement::new(
        "schema_history_stream_idx",
        "schema_history",
        false,
        &["org", "stream_type", "stream_name"],
    ))
    .await?;
    create_index(IndexStatement::new(
        "schema_history_stream_version_idx",
        "schema_history",
        true,
        &["org", "stream_type", "stream_name", "start_dt"],
    ))
    .await?;

    Ok(())
}
