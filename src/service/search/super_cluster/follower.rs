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

use std::sync::Arc;

use config::{
    cluster::LOCAL_NODE,
    meta::{
        cluster::{IntoArcVec, RoleGroup},
        search::{ScanStats, SearchEventType},
        sql::TableReferenceExt,
        stream::StreamType,
    },
};
use datafusion::{
    common::{TableReference, tree_node::TreeNode},
    physical_plan::ExecutionPlan,
    prelude::SessionContext,
};
use datafusion_proto::bytes::physical_plan_from_bytes_with_extension_codec;
use infra::{
    errors::{Error, Result},
    file_list::FileId,
};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::service::{
    db::enrichment_table,
    search::{
        cluster::flight::{check_work_group, get_online_querier_nodes, partition_filt_list},
        datafusion::{
            distributed_plan::{
                NewEmptyExecVisitor,
                codec::get_physical_extension_codec,
                empty_exec::NewEmptyExec,
                node::{RemoteScanNode, SearchInfos},
                remote_scan::RemoteScanExec,
            },
            exec::{prepare_datafusion_context, register_udf},
        },
        inspector::{SearchInspectorFieldsBuilder, search_inspector_fields},
        request::{FlightSearchRequest, Request},
        utils::AsyncDefer,
    },
};

/// in cluster search function only single stream take part in
// 1. get nodes
// 2. get file list
// 3. partition file list
// 4. get physical plan
// 5. add remote scan to physical plan
// 6. execute physical plan to get stream
#[tracing::instrument(name = "service:search:grpc:flight:follower:search", skip_all)]
pub async fn search(
    trace_id: &str,
    flight_request: &FlightSearchRequest,
) -> Result<(
    SessionContext,
    Arc<dyn ExecutionPlan>,
    AsyncDefer,
    ScanStats,
)> {
    let start = std::time::Instant::now();
    let cfg = config::get_config();
    let mut req: Request = (*flight_request).clone().into();
    let trace_id = trace_id.to_string();

    // create datafusion context, just used for decode plan, the params can use default
    let mut ctx = prepare_datafusion_context(
        &trace_id,
        req.work_group.clone(),
        vec![],
        vec![],
        false,
        cfg.limit.cpu_num,
    )
    .await?;

    // register udf
    register_udf(&ctx, &req.org_id)?;
    datafusion_functions_json::register_all(&mut ctx)?;

    // Decode physical plan from bytes
    let proto = get_physical_extension_codec();
    let mut physical_plan = physical_plan_from_bytes_with_extension_codec(
        &flight_request.search_info.plan,
        &ctx,
        &proto,
    )?;

    // replace empty table to real table
    let mut visitor = NewEmptyExecVisitor::default();
    if physical_plan.visit(&mut visitor).is_err() || visitor.get_data().is_none() {
        return Err(Error::Message(
            "flight->follower_leader: physical plan visit error: there is no EmptyTable"
                .to_string(),
        ));
    }
    let empty_exec = visitor
        .get_data()
        .unwrap()
        .as_any()
        .downcast_ref::<NewEmptyExec>()
        .unwrap();

    // get stream name
    let stream = TableReference::from(empty_exec.name());
    let stream_type = stream.get_stream_type(req.stream_type);

    // 1. get file id list
    let file_id_list =
        get_file_id_lists(&trace_id, &req.org_id, stream_type, &stream, req.time_range).await?;
    let file_id_list_vec = file_id_list.iter().collect::<Vec<_>>();
    let file_id_list_num = file_id_list_vec.len();
    let file_id_list_took = start.elapsed().as_millis() as usize;
    log::info!(
        "{}",
        search_inspector_fields(
            format!(
                "[trace_id {trace_id}] flight->follower_leader: get file_list time_range: {:?}, files: {}, took: {} ms",
                req.time_range, file_id_list_num, file_id_list_took,
            ),
            SearchInspectorFieldsBuilder::new()
                .node_name(LOCAL_NODE.name.clone())
                .component("super:leader get file id".to_string())
                .search_role("leader".to_string())
                .duration(file_id_list_took)
                .desc(format!("get files {file_id_list_num} ids"))
                .build()
        )
    );

    let mut scan_stats = ScanStats {
        files: file_id_list_num as i64,
        original_size: file_id_list_vec.iter().map(|v| v.original_size).sum(),
        file_list_took: file_id_list_took as i64,
        ..Default::default()
    };

    // get nodes
    let get_node_start = std::time::Instant::now();
    let role_group = req
        .search_event_type
        .as_ref()
        .map(|v| {
            SearchEventType::try_from(v.as_str())
                .ok()
                .map(RoleGroup::from)
        })
        .unwrap_or(Some(RoleGroup::Interactive));
    let mut nodes = get_online_querier_nodes(&trace_id, role_group).await?;

    // local mode, only use local node as querier node
    if req.local_mode.unwrap_or_default() {
        if LOCAL_NODE.is_querier() {
            nodes.retain(|n| n.name.eq(&LOCAL_NODE.name));
        } else {
            nodes = nodes
                .into_iter()
                .filter(|n| n.is_querier())
                .take(1)
                .collect();
        }
    }

    let querier_num = nodes.iter().filter(|node| node.is_querier()).count();
    if querier_num == 0 {
        log::error!("no querier node online");
        return Err(Error::Message("no querier node online".to_string()));
    }

    log::info!(
        "{}",
        search_inspector_fields(
            format!(
                "[trace_id {trace_id}] flight->follower_leader: get nodes num: {}, querier num: {}",
                nodes.len(),
                querier_num,
            ),
            SearchInspectorFieldsBuilder::new()
                .node_name(LOCAL_NODE.name.clone())
                .component("super:leader get nodes".to_string())
                .search_role("leader".to_string())
                .duration(get_node_start.elapsed().as_millis() as usize)
                .desc(format!(
                    "get nodes num: {}, querier num: {}",
                    nodes.len(),
                    querier_num
                ))
                .build()
        )
    );

    // check work group
    let (_took_wait, work_group_str, work_group) = check_work_group(
        &req,
        &trace_id,
        &nodes,
        &file_id_list_vec,
        start,
        file_id_list_took,
        "leader".to_string(),
    )
    .await?;
    // add work_group
    req.add_work_group(Some(work_group_str));

    // release work_group in flight follow search
    let user_id = req.user_id.clone();
    let trace_id_move = trace_id.to_string();
    let defer = AsyncDefer::new({
        async move {
            let _ = work_group
                .as_ref()
                .unwrap()
                .done(&trace_id_move, user_id.as_deref())
                .await
                .map_err(|e| {
                    log::error!(
                        "[trace_id {trace_id_move}] release work_group in flight follow search error: {e}",
                    );
                    e.to_string();
                });
            log::info!("[trace_id {trace_id_move}] release work_group in flight follow search");
        }
    });

    // partition file list
    let partition_file_lists = partition_filt_list(file_id_list, &nodes, role_group).await?;
    let mut need_ingesters = 0;
    let mut need_queriers = 0;
    for (i, node) in nodes.iter().enumerate() {
        if node.is_ingester() {
            need_ingesters += 1;
            continue;
        }
        if node.is_querier()
            && partition_file_lists
                .get(i)
                .map(|v| !v.is_empty())
                .unwrap_or_default()
        {
            need_queriers += 1;
        }
    }
    log::info!(
        "[trace_id {trace_id}] flight->follower_leader: get files num: {}, need ingester num: {}, need querier num: {}",
        file_id_list_num,
        need_ingesters,
        need_queriers,
    );

    // update search session scan stats
    super::super::SEARCH_SERVER
        .add_file_stats(
            &trace_id,
            scan_stats.files,
            scan_stats.records,
            scan_stats.original_size + scan_stats.idx_scan_size,
            scan_stats.compressed_size,
        )
        .await;

    let search_infos = SearchInfos {
        plan: vec![],
        file_id_list: partition_file_lists.clone(),
        start_time: req.time_range.as_ref().map(|x| x.0).unwrap_or(0),
        end_time: req.time_range.as_ref().map(|x| x.1).unwrap_or(0),
        timeout: req.timeout as u64,
        use_cache: req.use_cache,
        histogram_interval: req.histogram_interval,
    };

    let context = tracing::Span::current().context();
    let mut remote_scan_node = RemoteScanNode::from_flight_search_request(
        flight_request,
        search_infos,
        nodes.into_arc_vec(),
        context,
    );
    remote_scan_node.set_is_super_cluster(false);

    // add sort preserving merge node to preserving the order
    if physical_plan.name() == "SortPreservingMergeExec" {
        let top_merge_node = physical_plan.clone();
        let remote_scan_exec = Arc::new(RemoteScanExec::new(physical_plan, remote_scan_node)?);
        physical_plan = top_merge_node.with_new_children(vec![remote_scan_exec])?;
    } else {
        physical_plan = Arc::new(RemoteScanExec::new(physical_plan, remote_scan_node)?);
    }

    log::info!("[trace_id {trace_id}] flight->follower_leader: generate physical plan finish");

    // we should collect scan state by `collect_stats`, here need to reutrn empty for super cluster
    // follower
    scan_stats.files = 0;
    scan_stats.records = 0;
    scan_stats.original_size = 0;
    scan_stats.compressed_size = 0;

    Ok((ctx, physical_plan, defer, scan_stats))
}

#[tracing::instrument(
    name = "service:search:super_cluster:follower:get_file_id_lists",
    skip_all
)]
pub async fn get_file_id_lists(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    stream: &TableReference,
    mut time_range: Option<(i64, i64)>,
) -> Result<Vec<FileId>> {
    let stream_name = stream.stream_name();
    let stream_type = stream.get_stream_type(stream_type);
    // if stream is enrich, rewrite the time_range
    if let Some(schema) = stream.schema()
        && (schema == "enrich" || schema == "enrichment_tables")
    {
        let start = enrichment_table::get_start_time(org_id, &stream_name).await;
        let end = config::utils::time::now_micros();
        time_range = Some((start, end));
    }
    let file_id_list = crate::service::file_list::query_ids(
        trace_id,
        org_id,
        stream_type,
        &stream_name,
        time_range,
    )
    .await?;
    Ok(file_id_list)
}
