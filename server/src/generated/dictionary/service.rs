use aio_plugin_dictionary_model::{
    CreateDictionaryItemRequest, CreateDictionaryTypeRequest, DictionaryItem, DictionaryType,
    DictionaryView, UpdateDictionaryItemRequest, UpdateDictionaryTypeRequest,
};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait DictionaryService: Send + Sync {
    async fn initialize(&self) -> Result<()>;
    async fn view(&self, tenant_id: &str) -> Result<DictionaryView>;
    async fn create_type(
        &self,
        tenant_id: &str,
        request: CreateDictionaryTypeRequest,
    ) -> Result<DictionaryType>;
    async fn update_type(
        &self,
        tenant_id: &str,
        type_id: &str,
        request: UpdateDictionaryTypeRequest,
    ) -> Result<DictionaryType>;
    async fn delete_type(&self, tenant_id: &str, type_id: &str) -> Result<()>;
    async fn create_item(
        &self,
        tenant_id: &str,
        request: CreateDictionaryItemRequest,
    ) -> Result<DictionaryItem>;
    async fn update_item(
        &self,
        tenant_id: &str,
        item_id: &str,
        request: UpdateDictionaryItemRequest,
    ) -> Result<DictionaryItem>;
    async fn delete_item(&self, tenant_id: &str, item_id: &str) -> Result<()>;
}
