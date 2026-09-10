use std::env;

use aio_plugin_dictionary_model::{
    CreateDictionaryItemRequest, CreateDictionaryTypeRequest, DictionaryItem, DictionaryType,
    DictionaryView, UpdateDictionaryItemRequest, UpdateDictionaryTypeRequest,
};
use anyhow::{Context as _, Result, ensure};
use async_trait::async_trait;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

use super::{
    model::{DictionaryItemRow, DictionaryTypeRow},
    service::DictionaryService,
    util::{validate_item, validate_type},
};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS dictionary_types (
    tenant_id TEXT NOT NULL,
    id TEXT NOT NULL,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, id),
    UNIQUE (tenant_id, code)
);
CREATE TABLE IF NOT EXISTS dictionary_items (
    tenant_id TEXT NOT NULL,
    id TEXT NOT NULL,
    type_id TEXT NOT NULL,
    value TEXT NOT NULL,
    label TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_id, id),
    UNIQUE (tenant_id, type_id, value),
    FOREIGN KEY (tenant_id, type_id)
        REFERENCES dictionary_types (tenant_id, id)
        ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS dictionary_items_one_default
    ON dictionary_items (tenant_id, type_id)
    WHERE is_default;
"#;

#[derive(Debug)]
pub struct PostgresDictionaryService {
    pool: PgPool,
}

impl PostgresDictionaryService {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("AIO_DATABASE_URL")
            .or_else(|_| env::var("AZ_AIO_DATABASE_URL"))
            .context("字典插件缺少 AIO_DATABASE_URL")?;
        Self::from_database_url(&database_url)
    }

    pub fn from_database_url(database_url: &str) -> Result<Self> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(8)
                .connect_lazy(database_url)
                .context("创建字典数据库连接池失败")?,
        })
    }

    async fn ensure_type(&self, tenant_id: &str, type_id: &str) -> Result<()> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM dictionary_types WHERE tenant_id = $1 AND id = $2)",
        )
        .bind(tenant_id)
        .bind(type_id)
        .fetch_one(&self.pool)
        .await
        .context("检查字典类型失败")?;
        ensure!(exists, "字典类型不存在或不属于当前租户");
        Ok(())
    }
}

#[async_trait]
impl DictionaryService for PostgresDictionaryService {
    async fn initialize(&self) -> Result<()> {
        sqlx::raw_sql(SCHEMA)
            .execute(&self.pool)
            .await
            .context("创建字典插件数据表失败")?;
        Ok(())
    }

    async fn view(&self, tenant_id: &str) -> Result<DictionaryView> {
        let type_rows = sqlx::query_as::<_, (String, String, String, String, bool)>(
            "SELECT id, code, name, description, enabled FROM dictionary_types WHERE tenant_id = $1 ORDER BY name, code",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .context("读取字典类型失败")?;
        let item_rows = sqlx::query_as::<_, (String, String, String, String, i32, bool, bool)>(
            "SELECT id, type_id, value, label, sort_order, is_default, enabled FROM dictionary_items WHERE tenant_id = $1 ORDER BY type_id, sort_order, label, value",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await
        .context("读取字典项失败")?;
        let items = item_rows
            .into_iter()
            .map(|row| DictionaryItemRow {
                id: row.0,
                type_id: row.1,
                value: row.2,
                label: row.3,
                sort_order: row.4,
                is_default: row.5,
                enabled: row.6,
            })
            .collect::<Vec<_>>();
        let types = type_rows
            .into_iter()
            .map(|row| DictionaryTypeRow {
                id: row.0,
                code: row.1,
                name: row.2,
                description: row.3,
                enabled: row.4,
            })
            .map(|row| DictionaryType {
                items: items
                    .iter()
                    .filter(|item| item.type_id == row.id)
                    .map(|item| DictionaryItem {
                        id: item.id.clone(),
                        type_id: item.type_id.clone(),
                        value: item.value.clone(),
                        label: item.label.clone(),
                        sort_order: item.sort_order,
                        is_default: item.is_default,
                        enabled: item.enabled,
                    })
                    .collect(),
                id: row.id,
                code: row.code,
                name: row.name,
                description: row.description,
                enabled: row.enabled,
            })
            .collect();
        Ok(DictionaryView { types })
    }

    async fn create_type(
        &self,
        tenant_id: &str,
        request: CreateDictionaryTypeRequest,
    ) -> Result<DictionaryType> {
        validate_type(&request.code, &request.name, &request.description)?;
        let id = Uuid::new_v4().simple().to_string();
        let code = request.code.trim();
        let name = request.name.trim();
        let description = request.description.trim();
        sqlx::query(
            "INSERT INTO dictionary_types (tenant_id, id, code, name, description, enabled) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(tenant_id)
        .bind(&id)
        .bind(code)
        .bind(name)
        .bind(description)
        .bind(request.enabled)
        .execute(&self.pool)
        .await
        .context("创建字典类型失败")?;
        Ok(DictionaryType {
            id,
            code: code.to_owned(),
            name: name.to_owned(),
            description: description.to_owned(),
            enabled: request.enabled,
            items: Vec::new(),
        })
    }

    async fn update_type(
        &self,
        tenant_id: &str,
        type_id: &str,
        request: UpdateDictionaryTypeRequest,
    ) -> Result<DictionaryType> {
        validate_type(&request.code, &request.name, &request.description)?;
        let code = request.code.trim();
        let name = request.name.trim();
        let description = request.description.trim();
        let result = sqlx::query(
            "UPDATE dictionary_types SET code = $3, name = $4, description = $5, enabled = $6, updated_at = NOW() WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(type_id)
        .bind(code)
        .bind(name)
        .bind(description)
        .bind(request.enabled)
        .execute(&self.pool)
        .await
        .context("更新字典类型失败")?;
        ensure!(
            result.rows_affected() == 1,
            "字典类型不存在或不属于当前租户"
        );
        let items = self
            .view(tenant_id)
            .await?
            .types
            .into_iter()
            .find(|item| item.id == type_id)
            .context("更新后的字典类型不存在")?
            .items;
        Ok(DictionaryType {
            id: type_id.to_owned(),
            code: code.to_owned(),
            name: name.to_owned(),
            description: description.to_owned(),
            enabled: request.enabled,
            items,
        })
    }

    async fn delete_type(&self, tenant_id: &str, type_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM dictionary_types WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(type_id)
            .execute(&self.pool)
            .await
            .context("删除字典类型失败")?;
        ensure!(
            result.rows_affected() == 1,
            "字典类型不存在或不属于当前租户"
        );
        Ok(())
    }

    async fn create_item(
        &self,
        tenant_id: &str,
        request: CreateDictionaryItemRequest,
    ) -> Result<DictionaryItem> {
        validate_item(&request.value, &request.label, request.sort_order)?;
        self.ensure_type(tenant_id, &request.type_id).await?;
        let id = Uuid::new_v4().simple().to_string();
        let value = request.value.trim();
        let label = request.label.trim();
        let is_default = request.is_default && request.enabled;
        let mut transaction = self.pool.begin().await.context("开始创建字典项事务失败")?;
        if is_default {
            sqlx::query("UPDATE dictionary_items SET is_default = FALSE, updated_at = NOW() WHERE tenant_id = $1 AND type_id = $2 AND is_default")
                .bind(tenant_id)
                .bind(&request.type_id)
                .execute(&mut *transaction)
                .await
                .context("清除原默认字典项失败")?;
        }
        sqlx::query(
            "INSERT INTO dictionary_items (tenant_id, id, type_id, value, label, sort_order, is_default, enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(tenant_id)
        .bind(&id)
        .bind(&request.type_id)
        .bind(value)
        .bind(label)
        .bind(request.sort_order)
        .bind(is_default)
        .bind(request.enabled)
        .execute(&mut *transaction)
        .await
        .context("创建字典项失败")?;
        transaction
            .commit()
            .await
            .context("提交创建字典项事务失败")?;
        Ok(DictionaryItem {
            id,
            type_id: request.type_id,
            value: value.to_owned(),
            label: label.to_owned(),
            sort_order: request.sort_order,
            is_default,
            enabled: request.enabled,
        })
    }

    async fn update_item(
        &self,
        tenant_id: &str,
        item_id: &str,
        request: UpdateDictionaryItemRequest,
    ) -> Result<DictionaryItem> {
        validate_item(&request.value, &request.label, request.sort_order)?;
        self.ensure_type(tenant_id, &request.type_id).await?;
        let value = request.value.trim();
        let label = request.label.trim();
        let is_default = request.is_default && request.enabled;
        let mut transaction = self.pool.begin().await.context("开始更新字典项事务失败")?;
        if is_default {
            sqlx::query("UPDATE dictionary_items SET is_default = FALSE, updated_at = NOW() WHERE tenant_id = $1 AND type_id = $2 AND id <> $3 AND is_default")
                .bind(tenant_id)
                .bind(&request.type_id)
                .bind(item_id)
                .execute(&mut *transaction)
                .await
                .context("清除原默认字典项失败")?;
        }
        let result = sqlx::query(
            "UPDATE dictionary_items SET type_id = $3, value = $4, label = $5, sort_order = $6, is_default = $7, enabled = $8, updated_at = NOW() WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(item_id)
        .bind(&request.type_id)
        .bind(value)
        .bind(label)
        .bind(request.sort_order)
        .bind(is_default)
        .bind(request.enabled)
        .execute(&mut *transaction)
        .await
        .context("更新字典项失败")?;
        ensure!(result.rows_affected() == 1, "字典项不存在或不属于当前租户");
        transaction
            .commit()
            .await
            .context("提交更新字典项事务失败")?;
        Ok(DictionaryItem {
            id: item_id.to_owned(),
            type_id: request.type_id,
            value: value.to_owned(),
            label: label.to_owned(),
            sort_order: request.sort_order,
            is_default,
            enabled: request.enabled,
        })
    }

    async fn delete_item(&self, tenant_id: &str, item_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM dictionary_items WHERE tenant_id = $1 AND id = $2")
            .bind(tenant_id)
            .bind(item_id)
            .execute(&self.pool)
            .await
            .context("删除字典项失败")?;
        ensure!(result.rows_affected() == 1, "字典项不存在或不属于当前租户");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn persists_crud_with_tenant_isolation_and_single_default() -> Result<()> {
        let Ok(database_url) = env::var("AIO_TEST_DATABASE_URL") else {
            return Ok(());
        };
        let service = PostgresDictionaryService::from_database_url(&database_url)?;
        service.initialize().await?;
        let tenant = format!("dictionary-test-{}", Uuid::new_v4().simple());
        let other_tenant = format!("dictionary-test-{}", Uuid::new_v4().simple());
        let dictionary_type = service
            .create_type(
                &tenant,
                CreateDictionaryTypeRequest {
                    code: "order-status".to_owned(),
                    name: "订单状态".to_owned(),
                    description: "订单生命周期".to_owned(),
                    enabled: true,
                },
            )
            .await?;
        let first = service
            .create_item(
                &tenant,
                CreateDictionaryItemRequest {
                    type_id: dictionary_type.id.clone(),
                    value: "pending".to_owned(),
                    label: "待处理".to_owned(),
                    sort_order: 10,
                    is_default: true,
                    enabled: true,
                },
            )
            .await?;
        let second = service
            .create_item(
                &tenant,
                CreateDictionaryItemRequest {
                    type_id: dictionary_type.id.clone(),
                    value: "paid".to_owned(),
                    label: "已支付".to_owned(),
                    sort_order: 20,
                    is_default: true,
                    enabled: true,
                },
            )
            .await?;

        let view = service.view(&tenant).await?;
        assert_eq!(view.types.len(), 1);
        assert_eq!(view.types[0].items.len(), 2);
        assert!(
            !view.types[0]
                .items
                .iter()
                .find(|item| item.id == first.id)
                .unwrap()
                .is_default
        );
        assert!(
            view.types[0]
                .items
                .iter()
                .find(|item| item.id == second.id)
                .unwrap()
                .is_default
        );
        assert!(service.view(&other_tenant).await?.types.is_empty());

        service.delete_type(&tenant, &dictionary_type.id).await?;
        assert!(service.view(&tenant).await?.types.is_empty());
        Ok(())
    }
}
