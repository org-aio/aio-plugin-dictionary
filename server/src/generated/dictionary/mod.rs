mod controller;
mod model;
mod service;
mod service_impl;
mod util;

use anyhow::{Context as _, Result};
use axum::Router;
use dill::CatalogBuilder;

pub use service::DictionaryService;
pub use service_impl::PostgresDictionaryService;

pub(crate) fn register(builder: &mut CatalogBuilder) -> Result<()> {
    builder
        .add_value(PostgresDictionaryService::from_env()?)
        .bind::<dyn DictionaryService, PostgresDictionaryService>()
        .add::<controller::DictionaryController>();
    Ok(())
}

pub(crate) fn router(catalog: &dill::Catalog) -> Result<Router> {
    let controller = catalog
        .get_one::<controller::DictionaryController>()
        .context("字典 Controller 未注册")?;
    Ok(controller::router(controller))
}
