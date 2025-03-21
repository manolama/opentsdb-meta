/*
 *
 *  * This file is part of OpenTSDB.
 *  * Copyright (C) 2021  Yahoo.
 *  *
 *  * Licensed under the Apache License, Version 2.0 (the "License");
 *  * you may not use this file except in compliance with the License.
 *  * You may obtain a copy of the License at
 *  *
 *  *   http://www.apache.org/licenses/LICENSE-2.0
 *  *
 *  * Unless required by applicable law or agreed to in writing, software
 *  * distributed under the License is distributed on an "AS IS" BASIS,
 *  * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  * See the License for the specific language governing permissions and
 *  * limitations under the License.
 *
 */

extern crate myst;
use log::{error, info};
use myst::myst_grpc::myst_service_server::MystService;
use myst::myst_grpc::myst_service_server::MystServiceServer;
use myst::myst_grpc::QueryRequest;
use myst::myst_grpc::TimeseriesResponse;
use myst::query::cache::Cache;
use myst::s3::segment_download::start_download;
use myst::utils::config::Config;
use std::pin::Pin;

use tonic::{Request, Response, Status};

use myst::query::query::Query;
use myst::{init_yamas_metrics, setup_logger};
use std::sync::Arc;
use std::time::SystemTime;
use tokio_stream::Stream;
use tonic::transport::Server;

pub struct TimeseriesService {
    pub thread_pool: rayon::ThreadPool,
    pub cache: Arc<Cache>,
    pub config: Config,
}

impl Default for TimeseriesService {
    fn default() -> Self {
        Self {
            thread_pool: rayon::ThreadPoolBuilder::new()
                .num_threads(num_cpus::get())
                .build()
                .unwrap(),

            cache: Arc::new(Cache::new()),
            config: Config::new(),
        }
    }
}

#[tonic::async_trait]
impl MystService for TimeseriesService {
    type GetTimeseriesStream =
        Pin<Box<dyn Stream<Item = Result<TimeseriesResponse, Status>> + Send + Sync>>;

    async fn get_timeseries(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::GetTimeseriesStream>, tonic::Status> {
        let r = request.into_inner();
        let query = r.query;
        let curr_time = SystemTime::now();
        info!("Running query {:?}", query);
        let batch_query = Query::from_json(&query).unwrap();
        let res = Query::run_query(
            &batch_query,
            &self.thread_pool,
            self.cache.clone(),
            &self.config,
        );

        match res {
            Ok(res) => Ok(Response::new(Box::pin(
                tokio_stream::wrappers::ReceiverStream::new(res),
            ))),

            Err(_) => {
                error!("Error querying {:?} ", res);
                Err(tonic::Status::internal("Query failed"))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new();
    setup_logger(String::from(&config.log_file))?;

    init_yamas_metrics(&config);

    //YamasKafkaConsumer::start_consumers();
    start_download().await?;

    start_grpc_server().await
}

async fn start_grpc_server() -> Result<(), Box<dyn std::error::Error>> {
    let mut hostname = local_ipaddress::get().unwrap();
    hostname.push_str(":9999");
    let addr = hostname.parse()?;

    let myst_service = TimeseriesService::default();

    let svc = MystServiceServer::new(myst_service);

    info!("Starting server on {:?}", hostname);

    Server::builder().add_service(svc).serve(addr).await?;
    Ok(())
}
