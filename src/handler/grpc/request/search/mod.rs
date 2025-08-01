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

use config::{
    meta::{search, stream::StreamType},
    utils::json,
};
use proto::cluster_rpc::{
    CancelQueryRequest, CancelQueryResponse, DeleteResultRequest, DeleteResultResponse,
    GetResultRequest, GetResultResponse, GetScanStatsRequest, QueryStatusRequest,
    QueryStatusResponse, ScanStatsResponse, SearchPartitionRequest, SearchPartitionResponse,
    SearchRequest, SearchResponse, search_server::Search,
};
use tonic::{Request, Response, Status};
use tracing_opentelemetry::OpenTelemetrySpanExt;
#[cfg(feature = "enterprise")]
use {
    config::metrics,
    o2_enterprise::enterprise::search::{QueryManager, TaskStatus, WorkGroup},
    proto::cluster_rpc::ScanStats,
};

use crate::{handler::grpc::MetadataMap, service::search as SearchService};

#[derive(Clone, Debug)]
#[cfg(feature = "enterprise")]
pub struct Searcher {
    pub query_manager: std::sync::Arc<QueryManager>,
}

#[cfg(feature = "enterprise")]
impl Searcher {
    pub fn new() -> Self {
        Self {
            query_manager: std::sync::Arc::new(QueryManager::new()),
        }
    }

    // check is the trace_id in the query_manager
    pub async fn contain_key(&self, trace_id: &str) -> bool {
        self.query_manager.contain_key(trace_id).await
    }

    // insert the trace_id and task_status into the query_manager
    pub async fn insert(&self, trace_id: String, task_status: TaskStatus) {
        self.query_manager.insert(trace_id, task_status).await;
    }

    // remove the trace_id from the query_manager
    pub async fn remove(
        &self,
        trace_id: &str,
        query_cancelled: bool,
    ) -> Option<Vec<(String, TaskStatus)>> {
        self.query_manager.remove(trace_id, query_cancelled).await
    }

    // check is the trace_id in the query_manager and is_leader
    pub async fn is_leader(&self, trace_id: &str) -> bool {
        self.query_manager.is_leader(trace_id).await
    }

    // insert the sender into the query_manager by trace_id
    pub async fn insert_sender(
        &self,
        trace_id: &str,
        sender: tokio::sync::oneshot::Sender<()>,
        query_start: bool,
    ) -> Result<(), infra::errors::Error> {
        self.query_manager
            .insert_sender(trace_id, sender, query_start)
            .await
    }

    // get all task status that is leader
    pub async fn get_task_status(&self) -> Vec<proto::cluster_rpc::QueryStatus> {
        self.query_manager.get_task_status().await
    }

    // add file stats
    pub async fn add_file_stats(
        &self,
        trace_id: &str,
        files: i64,
        records: i64,
        original_size: i64,
        compressed_size: i64,
    ) {
        self.query_manager
            .add_file_stats(trace_id, files, records, original_size, compressed_size)
            .await;
    }

    // add work group
    pub async fn add_work_group(&self, trace_id: &str, work_group: Option<WorkGroup>) {
        self.query_manager
            .add_work_group(trace_id, work_group)
            .await;
    }

    pub async fn set_scan_stats(&self, trace_id: &str, stats: ScanStats) {
        self.query_manager
            .set_stats(trace_id, (&stats).into())
            .await;
    }

    pub async fn get_scan_stats(&self, trace_id: &str) -> ScanStats {
        (&self.query_manager.get_stats(trace_id).await).into()
    }
}

#[derive(Clone, Debug)]
#[cfg(not(feature = "enterprise"))]
pub struct Searcher;

#[cfg(not(feature = "enterprise"))]
impl Searcher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl Search for Searcher {
    #[tracing::instrument(name = "grpc:search:search", skip_all)]
    async fn search(
        &self,
        req: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let parent_cx = opentelemetry::global::get_text_map_propagator(|prop| {
            prop.extract(&MetadataMap(req.metadata()))
        });
        tracing::Span::current().set_parent(parent_cx.clone());

        let start = std::time::Instant::now();
        let req = req.into_inner();
        let request = json::from_slice::<search::Request>(&req.request)
            .map_err(|e| Status::internal(format!("failed to parse search request: {e}")))?;
        let stream_type = StreamType::from(req.stream_type.as_str());
        let ret = SearchService::cache::search(
            &req.trace_id,
            &req.org_id,
            stream_type,
            req.user_id.clone(),
            &request,
            "".to_string(),
            false,
        )
        .await;

        match ret {
            Ok(mut ret) => {
                ret.set_took(start.elapsed().as_millis() as usize);
                let response = json::to_vec(&ret).map_err(|e| {
                    Status::internal(format!("failed to serialize search response: {e}"))
                })?;
                Ok(Response::new(SearchResponse {
                    trace_id: req.trace_id,
                    response,
                }))
            }
            Err(e) => Err(Status::internal(format!("search failed: {e}"))),
        }
    }

    #[tracing::instrument(name = "grpc:search:search_multi", skip_all)]
    async fn search_multi(
        &self,
        req: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let parent_cx = opentelemetry::global::get_text_map_propagator(|prop| {
            prop.extract(&MetadataMap(req.metadata()))
        });
        tracing::Span::current().set_parent(parent_cx.clone());

        let req = req.into_inner();
        let request =
            json::from_slice::<search::MultiStreamRequest>(&req.request).map_err(|e| {
                Status::internal(format!("failed to parse multi-stream search request: {e}"))
            })?;
        let stream_type = StreamType::from(req.stream_type.as_str());
        let ret =
            SearchService::search_multi(&req.trace_id, &req.org_id, stream_type, None, &request)
                .await;

        match ret {
            Ok(ret) => {
                let response = json::to_vec(&ret).map_err(|e| {
                    Status::internal(format!("failed to serialize search response: {e}"))
                })?;
                Ok(Response::new(SearchResponse {
                    trace_id: req.trace_id,
                    response,
                }))
            }
            Err(e) => Err(Status::internal(format!("search failed: {e}"))),
        }
    }

    async fn search_partition(
        &self,
        req: Request<SearchPartitionRequest>,
    ) -> Result<Response<SearchPartitionResponse>, Status> {
        let req = req.into_inner();
        let request =
            json::from_slice::<search::SearchPartitionRequest>(&req.request).map_err(|e| {
                Status::internal(format!("failed to parse search partition request: {e}"))
            })?;
        let stream_type = StreamType::from(req.stream_type.as_str());
        let ret = SearchService::search_partition(
            &req.trace_id,
            &req.org_id,
            None,
            stream_type,
            &request,
            req.skip_max_query_range,
            true,
        )
        .await;

        match ret {
            Ok(ret) => {
                let response = json::to_vec(&ret).map_err(|e| {
                    Status::internal(format!(
                        "failed to serialize search partition response: {e}"
                    ))
                })?;
                Ok(Response::new(SearchPartitionResponse {
                    trace_id: req.trace_id,
                    response,
                }))
            }
            Err(e) => Err(Status::internal(format!("search partition failed: {e}"))),
        }
    }

    #[cfg(feature = "enterprise")]
    async fn get_result(
        &self,
        req: Request<GetResultRequest>,
    ) -> Result<Response<GetResultResponse>, Status> {
        let path = req.into_inner().path;
        let res = infra::storage::get_bytes("", &path)
            .await
            .map_err(|e| Status::internal(format!("failed to get result: {e}")))?;
        Ok(Response::new(GetResultResponse {
            response: res.to_vec(),
        }))
    }

    #[cfg(not(feature = "enterprise"))]
    async fn get_result(
        &self,
        _req: Request<GetResultRequest>,
    ) -> Result<Response<GetResultResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }

    #[cfg(feature = "enterprise")]
    async fn delete_result(
        &self,
        req: Request<DeleteResultRequest>,
    ) -> Result<Response<DeleteResultResponse>, Status> {
        let paths = req.into_inner().paths;
        let paths = paths
            .iter()
            .map(|path| ("", path.as_str()))
            .collect::<Vec<_>>();
        let _ = infra::storage::del(paths)
            .await
            .map_err(|e| Status::internal(format!("failed to delete result: {e}")))?;
        Ok(Response::new(DeleteResultResponse {}))
    }

    #[cfg(not(feature = "enterprise"))]
    async fn delete_result(
        &self,
        _req: Request<DeleteResultRequest>,
    ) -> Result<Response<DeleteResultResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }

    #[cfg(feature = "enterprise")]
    async fn query_status(
        &self,
        _req: Request<QueryStatusRequest>,
    ) -> Result<Response<QueryStatusResponse>, Status> {
        let status = self.get_task_status().await;
        Ok(Response::new(QueryStatusResponse { status }))
    }

    #[cfg(not(feature = "enterprise"))]
    async fn query_status(
        &self,
        _req: Request<QueryStatusRequest>,
    ) -> Result<Response<QueryStatusResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }

    #[cfg(feature = "enterprise")]
    async fn cancel_query(
        &self,
        req: Request<CancelQueryRequest>,
    ) -> Result<Response<CancelQueryResponse>, Status> {
        let trace_id = req.into_inner().trace_id;
        if let Some(cancelled) = self.remove(&trace_id, true).await {
            for (_, senders) in cancelled {
                for sender in senders.abort_senders.into_iter().rev() {
                    let _ = sender.send(());
                }
                metrics::QUERY_CANCELED_NUMS
                    .with_label_values(&[&senders.org_id.unwrap_or_default()])
                    .inc();
            }
        }
        Ok(Response::new(CancelQueryResponse { is_success: true }))
    }

    #[cfg(not(feature = "enterprise"))]
    async fn cancel_query(
        &self,
        _req: Request<CancelQueryRequest>,
    ) -> Result<Response<CancelQueryResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }

    #[cfg(feature = "enterprise")]
    async fn cluster_cancel_query(
        &self,
        req: Request<CancelQueryRequest>,
    ) -> Result<Response<CancelQueryResponse>, Status> {
        use crate::service::search as SearchService;

        let trace_id = req.into_inner().trace_id;
        if let Err(e) = SearchService::cancel_query("", &trace_id).await {
            log::error!("failed to cancel query: {e}");
        }
        Ok(Response::new(CancelQueryResponse { is_success: true }))
    }

    #[cfg(not(feature = "enterprise"))]
    async fn cluster_cancel_query(
        &self,
        _req: Request<CancelQueryRequest>,
    ) -> Result<Response<CancelQueryResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }

    #[cfg(feature = "enterprise")]
    async fn get_scan_stats(
        &self,
        req: Request<GetScanStatsRequest>,
    ) -> Result<Response<ScanStatsResponse>, Status> {
        use std::sync::Arc;

        use config::meta::cluster::NodeInfo;

        let inner_req = req.into_inner();

        let is_leader = inner_req.is_leader;
        let trace_id = inner_req.trace_id;
        if !is_leader {
            let stats = self.get_scan_stats(&trace_id).await;
            Ok(Response::new(ScanStatsResponse { stats: Some(stats) }))
        } else {
            let mut ret = search::ScanStats::default();
            if let Some(nodes) =
                crate::common::infra::cluster::get_cached_online_query_nodes(None).await
            {
                let nodes: Vec<_> = nodes
                    .into_iter()
                    .map(|n| Arc::new(n) as Arc<dyn NodeInfo>)
                    .collect();
                let stats =
                    crate::service::search::utils::collect_scan_stats(&nodes, &trace_id, false)
                        .await;
                ret.add(&stats);
            }
            Ok(Response::new(ScanStatsResponse {
                stats: Some((&ret).into()),
            }))
        }
    }

    #[cfg(not(feature = "enterprise"))]
    async fn get_scan_stats(
        &self,
        _req: Request<GetScanStatsRequest>,
    ) -> Result<Response<ScanStatsResponse>, Status> {
        Err(Status::unimplemented("Not Supported"))
    }
}
