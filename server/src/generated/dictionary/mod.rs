mod controller;
mod model;
mod service;
mod service_impl;
mod util;

use anyhow::{Context as _, Result};
use axum::Router;

pub use controller::DictionaryController;
pub use service::DictionaryService;
pub use service_impl::PostgresDictionaryService;

pub(crate) fn router(catalog: &dill::Catalog) -> Result<Router> {
    let controller = catalog
        .get_one::<controller::DictionaryController>()
        .context("字典 Controller 未注册")?;
    Ok(controller::router(controller))
}
