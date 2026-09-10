#![forbid(unsafe_code)]

mod generated;

use anyhow::{Context as _, Result};
use axum::Router;
use dill::CatalogBuilder;

pub use generated::dictionary::{DictionaryService, PostgresDictionaryService};

pub fn register(builder: &mut CatalogBuilder) -> Result<()> {
    generated::dictionary::register(builder)
}

pub fn service(catalog: &dill::Catalog) -> Result<std::sync::Arc<dyn DictionaryService>> {
    catalog
        .get_one::<dyn DictionaryService>()
        .context("字典服务未注册")
}

pub fn router(catalog: &dill::Catalog) -> Result<Router> {
    generated::dictionary::router(catalog)
}
