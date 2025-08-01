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
    collections::{HashMap, HashSet},
    io::Write,
    sync::Arc,
    time::Instant,
};

use arrow_schema::{DataType, Field};
use bulk::SCHEMA_CONFORMANCE_FAILED;
use config::{
    DISTINCT_FIELDS, SIZE_IN_MB, get_config,
    meta::{
        alerts::alert::Alert,
        self_reporting::usage::{RequestStats, UsageType},
        stream::{PartitionTimeLevel, StreamParams, StreamPartition, StreamType},
    },
    metrics,
    utils::{
        json::{Map, Value, estimate_json_bytes, get_string_value, pickup_string_value},
        schema_ext::SchemaExt,
        time::now_micros,
    },
};
use infra::{
    errors::{Error, Result},
    schema::{SchemaCache, unwrap_partition_time_level},
};

use super::{
    db::organization::get_org_setting,
    ingestion::{TriggerAlertData, evaluate_trigger, write_file},
    metadata::{
        MetadataItem, MetadataType,
        distinct_values::{DISTINCT_STREAM_PREFIX, DvItem},
        write,
    },
    schema::stream_schema_exists,
};
use crate::{
    common::meta::{ingestion::IngestionStatus, stream::SchemaRecords},
    service::{
        alerts::alert::AlertExt, db, ingestion::get_write_partition_key, schema::check_for_schema,
        self_reporting::report_request_usage_stats,
    },
};

pub mod bulk;
pub mod hec;
pub mod ingest;
pub mod loki;
pub mod otlp;
pub mod syslog;

static BULK_OPERATORS: [&str; 3] = ["create", "index", "update"];

pub type O2IngestJsonData = (Vec<(i64, Map<String, Value>)>, Option<usize>);

fn parse_bulk_index(v: &Value) -> Option<(String, String, Option<String>)> {
    let local_val = v.as_object().unwrap();
    for action in BULK_OPERATORS {
        if let Some(val) = local_val.get(action) {
            let Some(local_val) = val.as_object() else {
                log::warn!("Invalid bulk index action: {action}");
                continue;
            };
            let Some(index) = local_val
                .get("_index")
                .and_then(|v| v.as_str().map(|v| v.to_string()))
            else {
                continue;
            };
            let doc_id = local_val
                .get("_id")
                .and_then(|v| v.as_str().map(|v| v.to_string()));
            return Some((action.to_string(), index, doc_id));
        };
    }
    None
}

pub fn cast_to_type(
    value: &mut Map<String, Value>,
    delta: Vec<Field>,
) -> Result<(), anyhow::Error> {
    let mut parse_error = String::new();
    for field in delta {
        let field_name = field.name().clone();
        let Some(val) = value.get(&field_name) else {
            continue;
        };
        if val.is_null() {
            value.insert(field_name, Value::Null);
            continue;
        }
        match field.data_type() {
            DataType::Utf8 => {
                if val.is_string() {
                    continue;
                }
                value.insert(field_name, Value::String(get_string_value(val)));
            }
            DataType::Int64 | DataType::Int32 | DataType::Int16 | DataType::Int8 => {
                let ret = match val {
                    Value::Number(_) => {
                        continue;
                    }
                    Value::String(v) => v.parse::<i64>().map_err(|e| e.to_string()),
                    Value::Bool(v) => Ok(if *v { 1 } else { 0 }),
                    _ => Err("".to_string()),
                };
                match ret {
                    Ok(val) => {
                        value.insert(field_name, Value::Number(val.into()));
                    }
                    Err(_) => set_parsing_error(&mut parse_error, &field),
                };
            }
            DataType::UInt64 | DataType::UInt32 | DataType::UInt16 | DataType::UInt8 => {
                let ret = match val {
                    Value::Number(_) => {
                        continue;
                    }
                    Value::String(v) => v.parse::<u64>().map_err(|e| e.to_string()),
                    Value::Bool(v) => Ok(if *v { 1 } else { 0 }),
                    _ => Err("".to_string()),
                };
                match ret {
                    Ok(val) => {
                        value.insert(field_name, Value::Number(val.into()));
                    }
                    Err(_) => set_parsing_error(&mut parse_error, &field),
                };
            }
            DataType::Float64 | DataType::Float32 | DataType::Float16 => {
                let ret = match val {
                    Value::Number(_) => {
                        continue;
                    }
                    Value::String(v) => v.parse::<f64>().map_err(|e| e.to_string()),
                    Value::Bool(v) => Ok(if *v { 1.0 } else { 0.0 }),
                    _ => Err("".to_string()),
                };
                match ret {
                    Ok(val) => {
                        value.insert(
                            field_name,
                            Value::Number(serde_json::Number::from_f64(val).unwrap()),
                        );
                    }
                    Err(_) => set_parsing_error(&mut parse_error, &field),
                };
            }
            DataType::Boolean => {
                let ret = match val {
                    Value::Bool(_) => {
                        continue;
                    }
                    Value::Number(v) => Ok(v.as_f64().unwrap_or(0.0) > 0.0),
                    Value::String(v) => v.parse::<bool>().map_err(|e| e.to_string()),
                    _ => Err("".to_string()),
                };
                match ret {
                    Ok(val) => {
                        value.insert(field_name, Value::Bool(val));
                    }
                    Err(_) => set_parsing_error(&mut parse_error, &field),
                };
            }
            _ => set_parsing_error(&mut parse_error, &field),
        };
    }
    if !parse_error.is_empty() {
        Err(anyhow::Error::msg(parse_error))
    } else {
        Ok(())
    }
}

fn set_parsing_error(parse_error: &mut String, field: &Field) {
    parse_error.push_str(&format!(
        "Failed to cast {} to type {} ",
        field.name(),
        field.data_type()
    ));
}

#[allow(clippy::too_many_arguments)]
async fn write_logs_by_stream(
    thread_id: usize,
    org_id: &str,
    user_email: &str,
    time_stats: (i64, &Instant), // started_at
    usage_type: UsageType,
    status: &mut IngestionStatus,
    json_data_by_stream: HashMap<String, O2IngestJsonData>,
    byte_size_by_stream: HashMap<String, usize>,
    derived_streams: HashSet<String>,
) -> Result<()> {
    for (stream_name, (json_data, fn_num)) in json_data_by_stream {
        // check if we are allowed to ingest
        if db::compact::retention::is_deleting_stream(org_id, StreamType::Logs, &stream_name, None)
        {
            log::warn!("stream [{stream_name}] is being deleted");
            continue; // skip
        }

        // write json data by stream
        let mut req_stats = write_logs(
            thread_id,
            org_id,
            &stream_name,
            status,
            json_data,
            derived_streams.contains(&stream_name),
        )
        .await?;

        let time_took = time_stats.1.elapsed().as_secs_f64();
        req_stats.response_time = time_took;
        req_stats.user_email = if user_email.is_empty() {
            None
        } else {
            Some(user_email.to_string())
        };

        req_stats.dropped_records = match status {
            IngestionStatus::Record(s) => s.failed.into(),
            IngestionStatus::Bulk(s) => {
                if s.errors {
                    s.items
                        .iter()
                        .map(|i| {
                            i.values()
                                .map(|res| if res.error.is_some() { 1 } else { 0 })
                                .sum::<i64>()
                        })
                        .sum()
                } else {
                    0
                }
            }
        };

        if let Some(fns_length) = fn_num {
            // the issue here is req_stats.size calculates size after flattening and
            // adding _timestamp col etc ; which inflates the size compared to the actual
            // data sent by user. So when reporting we check if the calling function has provided us
            // an "actual" size of the input, and is so use that instead of the req_stats
            if let Some(size) = byte_size_by_stream.get(&stream_name) {
                // req_stats already divides the size in mb
                req_stats.size = *size as f64 / SIZE_IN_MB;
            }
            report_request_usage_stats(
                req_stats,
                org_id,
                &stream_name,
                StreamType::Logs,
                usage_type,
                fns_length as u16,
                time_stats.0,
            )
            .await;
        }
    }
    Ok(())
}

async fn write_logs(
    thread_id: usize,
    org_id: &str,
    stream_name: &str,
    status: &mut IngestionStatus,
    json_data: Vec<(i64, Map<String, Value>)>,
    is_derived: bool,
) -> Result<RequestStats> {
    let cfg = get_config();
    let log_ingest_errors = ingestion_log_enabled().await;
    // get schema and stream settings
    let mut stream_schema_map: HashMap<String, SchemaCache> = HashMap::new();
    let stream_schema = stream_schema_exists(
        org_id,
        stream_name,
        StreamType::Logs,
        &mut stream_schema_map,
    )
    .await;

    let schema = match stream_schema_map.get(stream_name) {
        Some(schema) => schema.schema().clone(),
        None => {
            return Err(Error::IngestionError(format!(
                "Schema not found for stream: {stream_name}"
            )));
        }
    };
    let stream_settings = infra::schema::unwrap_stream_settings(&schema).unwrap_or_default();

    let mut partition_keys: Vec<StreamPartition> = vec![];
    let mut partition_time_level = PartitionTimeLevel::from(cfg.limit.logs_file_retention.as_str());
    if stream_schema.has_partition_keys {
        partition_keys = stream_settings.partition_keys;
        partition_time_level =
            unwrap_partition_time_level(stream_settings.partition_time_level, StreamType::Logs);
    }

    // Start get stream alerts
    let mut stream_alerts_map: HashMap<String, Vec<Alert>> = HashMap::new();
    crate::service::ingestion::get_stream_alerts(
        &[StreamParams {
            org_id: org_id.to_owned().into(),
            stream_name: stream_name.to_owned().into(),
            stream_type: StreamType::Logs,
        }],
        &mut stream_alerts_map,
    )
    .await;
    let cur_stream_alerts =
        stream_alerts_map.get(&format!("{}/{}/{}", org_id, StreamType::Logs, stream_name));
    let mut triggers: TriggerAlertData =
        Vec::with_capacity(cur_stream_alerts.map_or(0, |v| v.len()));
    let mut evaluated_alerts = HashSet::new();
    // End get stream alert

    // start check for schema
    let min_timestamp = json_data.iter().map(|(ts, _)| ts).min().unwrap();
    let (schema_evolution, infer_schema) = check_for_schema(
        org_id,
        stream_name,
        StreamType::Logs,
        &mut stream_schema_map,
        json_data.iter().map(|(_, v)| v).collect(),
        *min_timestamp,
        is_derived, // is_derived is true if the stream is derived
    )
    .await?;

    // get schema
    let latest_schema = stream_schema_map
        .get(stream_name)
        .unwrap()
        .schema()
        .as_ref()
        .clone()
        .with_metadata(HashMap::new());
    let schema_key = latest_schema.hash_key();
    // use latest schema as schema key
    // use inferred schema as record schema
    let rec_schema = match infer_schema {
        // use latest_schema's datetype for record schema
        Some(schema) => Arc::new(schema.cloned_from(&latest_schema)),
        None => Arc::new(latest_schema),
    };

    let mut distinct_values = Vec::with_capacity(16);

    let mut write_buf: HashMap<String, SchemaRecords> = HashMap::new();

    for (timestamp, mut record_val) in json_data {
        let doc_id = record_val
            .get("_id")
            .map(|v| v.as_str().unwrap().to_string());

        // validate record
        if let Some(delta) = schema_evolution.types_delta.as_ref() {
            let ret_val = if !schema_evolution.is_schema_changed {
                cast_to_type(&mut record_val, delta.to_owned())
            } else {
                let local_delta = delta
                    .iter()
                    .filter_map(|x| {
                        if x.metadata().contains_key("zo_cast") {
                            Some(x.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                if !local_delta.is_empty() {
                    cast_to_type(&mut record_val, local_delta)
                } else {
                    Ok(())
                }
            };
            if let Err(e) = ret_val {
                // update status(fail)
                match status {
                    IngestionStatus::Record(status) => {
                        status.failed += 1;
                        status.error = e.to_string();
                        metrics::INGEST_ERRORS
                            .with_label_values(&[
                                org_id,
                                StreamType::Logs.as_str(),
                                stream_name,
                                SCHEMA_CONFORMANCE_FAILED,
                            ])
                            .inc();
                        log_failed_record(log_ingest_errors, &record_val, &e.to_string());
                    }
                    IngestionStatus::Bulk(bulk_res) => {
                        bulk_res.errors = true;
                        metrics::INGEST_ERRORS
                            .with_label_values(&[
                                org_id,
                                StreamType::Logs.as_str(),
                                stream_name,
                                SCHEMA_CONFORMANCE_FAILED,
                            ])
                            .inc();
                        log_failed_record(log_ingest_errors, &record_val, &e.to_string());
                        bulk::add_record_status(
                            stream_name.to_string(),
                            &doc_id,
                            "".to_string(),
                            Some(Value::Object(record_val.clone())),
                            bulk_res,
                            Some(bulk::SCHEMA_CONFORMANCE_FAILED.to_string()),
                            Some(e.to_string()),
                        );
                    }
                }
                continue;
            }
        }

        // start check for alert trigger
        if let Some(alerts) = cur_stream_alerts
            && triggers.len() < alerts.len()
        {
            let end_time = now_micros();
            for alert in alerts {
                let key = format!(
                    "{}/{}/{}/{}",
                    org_id,
                    StreamType::Logs,
                    alert.stream_name,
                    alert.get_unique_key()
                );
                // For one alert, only one trigger per request
                // Trigger for this alert is already added.
                if evaluated_alerts.contains(&key) {
                    continue;
                }
                match alert
                    .evaluate(Some(&record_val), (None, end_time), None)
                    .await
                {
                    Ok(trigger_results) if trigger_results.data.is_some() => {
                        triggers.push((alert.clone(), trigger_results.data.unwrap()));
                        evaluated_alerts.insert(key);
                    }
                    _ => {}
                }
            }
        }
        // end check for alert triggers

        // get distinct_value items
        let mut map = Map::new();
        for field in DISTINCT_FIELDS.iter().chain(
            stream_settings
                .distinct_value_fields
                .iter()
                .map(|f| &f.name),
        ) {
            if let Some(val) = record_val.get(field) {
                map.insert(field.clone(), val.clone());
            }
        }

        if !map.is_empty() {
            // add distinct values
            distinct_values.push(MetadataItem::DistinctValues(DvItem {
                stream_type: StreamType::Logs,
                stream_name: stream_name.to_string(),
                value: map,
            }));
        }

        // get hour key
        let hour_key = get_write_partition_key(
            timestamp,
            &partition_keys,
            partition_time_level,
            &record_val,
            Some(&schema_key),
        );

        let hour_buf = write_buf.entry(hour_key).or_insert_with(|| SchemaRecords {
            schema_key: schema_key.clone(),
            schema: rec_schema.clone(),
            records: vec![],
            records_size: 0,
        });
        let record_val = Value::Object(record_val);
        let record_size = estimate_json_bytes(&record_val);
        hour_buf.records.push(Arc::new(record_val));
        hour_buf.records_size += record_size;

        // update status(success)
        match status {
            IngestionStatus::Record(status) => {
                status.successful += 1;
            }
            IngestionStatus::Bulk(bulk_res) => {
                bulk::add_record_status(
                    stream_name.to_string(),
                    &doc_id,
                    "".to_string(),
                    None,
                    bulk_res,
                    None,
                    None,
                );
            }
        }
    }

    // write data to wal
    let writer =
        ingester::get_writer(thread_id, org_id, StreamType::Logs.as_str(), stream_name).await;
    let req_stats = write_file(
        &writer,
        stream_name,
        write_buf,
        !cfg.common.wal_fsync_disabled,
    )
    .await?;

    // send distinct_values
    if !distinct_values.is_empty()
        && !stream_name.starts_with(DISTINCT_STREAM_PREFIX)
        && let Err(e) = write(org_id, MetadataType::DistinctValues, distinct_values).await
    {
        log::error!("Error while writing distinct values: {e}");
    }

    // only one trigger per request
    evaluate_trigger(triggers).await;

    Ok(req_stats)
}

pub fn refactor_map(
    original_map: Map<String, Value>,
    defined_schema_keys: &HashSet<String>,
) -> Map<String, Value> {
    let mut new_map = Map::with_capacity(defined_schema_keys.len() + 2);
    let mut non_schema_map = Vec::with_capacity(1024); // 1KB

    let mut has_elements = false;
    non_schema_map.write_all(b"{").unwrap();
    for (key, value) in original_map {
        if defined_schema_keys.contains(&key) {
            new_map.insert(key, value);
        } else {
            if has_elements {
                non_schema_map.write_all(b",").unwrap();
            } else {
                has_elements = true;
            }
            non_schema_map.write_all(b"\"").unwrap();
            non_schema_map.write_all(key.as_bytes()).unwrap();
            non_schema_map.write_all(b"\":\"").unwrap();
            non_schema_map
                .write_all(pickup_string_value(value).as_bytes())
                .unwrap();
            non_schema_map.write_all(b"\"").unwrap();
        }
    }
    non_schema_map.write_all(b"}").unwrap();

    if has_elements {
        new_map.insert(
            get_config().common.column_all.to_string(),
            Value::String(String::from_utf8(non_schema_map).unwrap()),
        );
    }

    new_map
}

async fn ingestion_log_enabled() -> bool {
    // the logging will be enabled through meta only, so hardcoded
    match get_org_setting("_meta").await {
        Ok(org_settings) => org_settings.toggle_ingestion_logs,
        Err(_) => false,
    }
}

fn log_failed_record<T: std::fmt::Debug>(enabled: bool, record: &T, error: &str) {
    if !enabled {
        return;
    }
    log::warn!("failed to process record with error {error} : {record:?} ");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_parsing_error() {
        let mut parse_error = String::new();
        set_parsing_error(&mut parse_error, &Field::new("test", DataType::Utf8, true));
        assert!(!parse_error.is_empty());
    }

    #[test]
    fn test_cast_to_type() {
        let mut local_val = Map::new();
        local_val.insert("test".to_string(), Value::from("test13212"));
        let delta = vec![Field::new("test", DataType::Utf8, true)];
        let ret_val = cast_to_type(&mut local_val, delta);
        assert!(ret_val.is_ok());
    }
}
