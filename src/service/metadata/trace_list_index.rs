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

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use arrow_schema::{DataType, Field, Schema};
use config::{
    TIMESTAMP_COL_NAME, get_config,
    meta::stream::{StreamPartition, StreamSettings, StreamType},
    utils::{json, schema_ext::SchemaExt, time::now_micros},
};
use infra::schema::unwrap_partition_time_level;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::{
    common::meta::stream::SchemaRecords,
    service::{
        db, ingestion,
        metadata::{Metadata, MetadataItem},
        stream,
    },
};

const STREAM_NAME: &str = "trace_list_index";

static PARTITION_KEYS: Lazy<[StreamPartition; 1]> =
    Lazy::new(|| [StreamPartition::new("service_name")]);

pub(crate) static INSTANCE: Lazy<TraceListIndex> = Lazy::new(TraceListIndex::new);

pub struct TraceListIndex {
    schema: Arc<Schema>,
    db_schema_init: AtomicBool,
}

#[derive(Debug, Default, Eq, Hash, PartialEq, Clone, Serialize, Deserialize)]
pub struct TraceListItem {
    pub _timestamp: i64,
    pub stream_name: String,
    pub service_name: String,
    pub trace_id: String,
}

impl Metadata for TraceListIndex {
    fn generate_schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new(TIMESTAMP_COL_NAME, DataType::Int64, false),
            Field::new("stream_name", DataType::Utf8, false),
            Field::new("service_name", DataType::Utf8, false),
            Field::new("trace_id", DataType::Utf8, false),
        ]))
    }

    async fn write(&self, org_id: &str, items: Vec<MetadataItem>) -> infra::errors::Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        // write to wal
        let timestamp = now_micros();
        let schema_key = self.schema.hash_key();

        let mut _is_new = false;
        if !self.db_schema_init.load(Ordering::Relaxed) {
            _is_new = self.set_db_schema(org_id).await?
        }

        let mut buf: HashMap<String, SchemaRecords> = HashMap::new();
        for item in items {
            let item = match item {
                MetadataItem::TraceListIndexer(item) => item,
                _ => {
                    continue;
                }
            };

            let mut data = json::to_value(item).unwrap();
            let data = data.as_object_mut().unwrap();
            let hour_key = ingestion::get_write_partition_key(
                timestamp,
                PARTITION_KEYS.to_vec().as_ref(),
                unwrap_partition_time_level(None, StreamType::Metadata),
                data,
                Some(&schema_key),
            );
            let data = json::Value::Object(data.clone());
            let data_size = json::to_vec(&data).unwrap_or_default().len();

            let hour_buf = buf.entry(hour_key).or_insert_with(|| SchemaRecords {
                schema_key: schema_key.clone(),
                schema: self.schema.clone(),
                records: vec![],
                records_size: 0,
            });

            hour_buf.records.push(Arc::new(data));
            hour_buf.records_size += data_size;
        }

        let writer =
            ingester::get_writer(0, org_id, StreamType::Metadata.as_str(), STREAM_NAME).await;
        _ = ingestion::write_file(
            &writer,
            STREAM_NAME,
            buf,
            !get_config().common.wal_fsync_disabled,
        )
        .await;

        #[cfg(feature = "enterprise")]
        {
            use o2_openfga::{
                authorizer::authz::set_ownership_if_not_exists,
                config::get_config as get_openfga_config,
            };

            // set ownership only in the first time
            if _is_new && get_openfga_config().enabled {
                set_ownership_if_not_exists(
                    org_id,
                    &format!("{}:{}", StreamType::Metadata, STREAM_NAME),
                )
                .await;
            }
        }

        Ok(())
    }
    async fn flush(&self) -> infra::errors::Result<()> {
        Ok(()) // do nothing
    }
    async fn stop(&self) -> infra::errors::Result<()> {
        if let Err(e) = self.flush().await {
            log::error!("[TraceListIndex] flush error: {e}");
        }
        Ok(())
    }
}

impl Default for TraceListIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceListIndex {
    pub fn new() -> Self {
        let mut res = Self {
            schema: Arc::new(Schema {
                fields: Default::default(),
                metadata: Default::default(),
            }),
            db_schema_init: AtomicBool::new(false),
        };

        res.schema = res.generate_schema();
        res
    }

    async fn set_db_schema(&self, org_id: &str) -> infra::errors::Result<bool> {
        // check for schema
        let db_schema = infra::schema::get(org_id, STREAM_NAME, StreamType::Metadata)
            .await
            .unwrap();
        let mut is_new = false;
        if db_schema.fields().is_empty() {
            is_new = true;
            let timestamp = now_micros();
            let schema = self.schema.as_ref().clone();
            if let Err(e) = db::schema::merge(
                org_id,
                STREAM_NAME,
                StreamType::Metadata,
                &schema,
                Some(timestamp),
            )
            .await
            {
                log::error!("[TraceListIndex] error while setting schema: {e}");
            }

            let settings = StreamSettings {
                partition_time_level: None,
                partition_keys: PARTITION_KEYS.to_vec(),
                full_text_search_keys: vec![],
                index_fields: vec![],
                bloom_filter_fields: vec!["trace_id".to_string()],
                data_retention: 0,
                flatten_level: None,
                max_query_range: 0,
                defined_schema_fields: vec![],
                store_original_data: false,
                approx_partition: false,
                distinct_value_fields: vec![],
                index_updated_at: 0,
                extended_retention_days: vec![],
                index_all_values: false,
                index_original_data: false,
            };

            stream::save_stream_settings(org_id, STREAM_NAME, StreamType::Metadata, settings)
                .await?;
        }

        self.db_schema_init.store(true, Ordering::Release);

        Ok(is_new)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use config::{
        meta::stream::StreamType,
        utils::{json, time::now_micros},
    };
    use infra::schema::unwrap_partition_time_level;

    use crate::{
        common::meta::stream::SchemaRecords,
        service::{
            ingestion,
            metadata::{
                Metadata, MetadataItem,
                trace_list_index::{STREAM_NAME, TraceListIndex, TraceListItem},
            },
        },
    };

    #[tokio::test]
    async fn test_write() {
        let t = TraceListIndex::new();
        let data = vec![MetadataItem::TraceListIndexer(TraceListItem::default())];

        let res = t.write("default", data).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_trace_list_index_write_file() {
        let t = TraceListIndex::new();
        let mut buf: HashMap<String, SchemaRecords> = HashMap::new();
        let item = TraceListItem {
            stream_name: "default".to_string(),
            service_name: "oojaeger".to_string(),
            trace_id: "b09e986672880927996155acd4ef113c".to_string(),
            _timestamp: 1711267573271714542,
        };
        let schema_key = "9d384d5af30d1657";
        let timestamp = now_micros();
        let mut data = json::to_value(item).unwrap();
        let data = data.as_object_mut().unwrap();
        data.insert(
            config::TIMESTAMP_COL_NAME.to_string(),
            json::Value::Number(timestamp.into()),
        );
        let hour_key = ingestion::get_write_partition_key(
            timestamp,
            &vec![],
            unwrap_partition_time_level(None, StreamType::Metadata),
            data,
            Some(schema_key),
        );
        let data = json::Value::Object(data.clone());
        let data_size = json::to_vec(&data).unwrap_or_default().len();
        let schema = t.generate_schema();
        let hour_buf = buf.entry(hour_key).or_insert_with(|| SchemaRecords {
            schema_key: schema_key.to_string(),
            schema,
            records: vec![],
            records_size: 0,
        });
        hour_buf.records.push(Arc::new(data));
        hour_buf.records_size += data_size;

        let writer =
            ingester::get_writer(0, "openobserve", StreamType::Metadata.as_str(), STREAM_NAME)
                .await;
        for (key, val) in buf.iter() {
            println!(
                "key: {key} val: {:?} schema: {}, records_size: {}, records: {:?}",
                val.schema_key, val.schema, val.records_size, val.records
            );
        }
        let r = ingestion::write_file(&writer, STREAM_NAME, buf, false).await;
        println!("r: {r:?}");
    }
}
