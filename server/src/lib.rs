#![forbid(unsafe_code)]

mod generated;
pub mod hosting;
pub mod migration;

pub use generated::dictionary::{
    DictionaryController, DictionaryService, PostgresDictionaryService,
};

/// 将现有 Controller 装配到进程的同一个 Dill 容器。
pub fn router(catalog: &dill::Catalog) -> anyhow::Result<axum::Router> {
    generated::dictionary::router(catalog)
}
