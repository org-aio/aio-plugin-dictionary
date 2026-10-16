//! 进程入口只接受宿主验证后的请求，身份和权限仍由运行时管理。

use crate::{DictionaryController, DictionaryService, PostgresDictionaryService};
use anyhow::{Context, Result};
use axum::{Json, routing::get};
use az_plugin_process::Host;
use serde_json::{Value, json};
use std::sync::Arc;

/// 装配现有业务服务和 Controller；受限数据库的表由整包迁移创建。
pub async fn run() -> Result<()> {
    let host = Host::from_env()?;
    let database = host
        .configuration
        .database_url
        .as_deref()
        .context("宿主未授权字典数据库")?;
    let service = Arc::new(PostgresDictionaryService::from_database_url(database)?);
    let result = async {
        service
            .verify_host_database(&host.configuration.tenant_id)
            .await?;
        let mut builder = dill::CatalogBuilder::new();
        builder
            .add_builder(service.clone())
            .bind::<dyn DictionaryService, PostgresDictionaryService>()
            .add::<DictionaryController>();
        let router = host
            .protect(crate::router(&builder.build())?)
            .route("/health", get(|| async { "ok" }))
            .route("/aio/describe", get(describe));
        host.serve(router).await
    }
    .await;
    service.shutdown().await;
    result
}

/// 页面定义随整包交付，宿主按安装版本动态展示和移除入口。
pub async fn describe() -> Json<Value> {
    Json(json!({"label":"字典管理","pages":[{
        "id":"dictionary-management", "label":"字典管理", "entry":"index.html",
        "scene":["system","系统"], "menu_path":[],
        "permission":null, "surface":"workspace"
    }]}))
}
