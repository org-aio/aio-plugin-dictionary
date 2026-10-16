//! 一次性从旧宿主复制字典到已安装整包的隔离 schema；不删除旧数据、不覆盖冲突。

use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sqlx::PgPool;

/// 复制结果，记录成功核对的类型和字典项数量。
#[derive(Debug, PartialEq, Eq)]
pub struct MigrationReport {
    pub types: usize,
    pub items: usize,
}

/// 运维工具调用。两个连接均由部署者提供，生产操作前停止旧宿主并完成数据库备份。
pub async fn copy_legacy(host: &PgPool, storage: &PgPool, tenant: &str) -> Result<MigrationReport> {
    ensure!(!tenant.is_empty() && tenant.len() <= 128, "租户无效");
    let source: uuid::Uuid = sqlx::query_scalar("SELECT id FROM component_sources WHERE git = $1")
        .bind("https://github.com/org-aio/aio-plugin-dictionary.git")
        .fetch_optional(host)
        .await?
        .context("请先发布并安装字典整包")?;
    let mut old = host.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *old)
        .await?;
    sqlx::query("LOCK TABLE public.dictionary_types, public.dictionary_items IN SHARE MODE")
        .execute(&mut *old)
        .await
        .context("锁定旧字典失败，请先停止旧宿主")?;
    let mut target = storage.begin().await?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *target)
        .await?;
    // 与运行时结构迁移使用相同锁，复制期间不能变更目标表契约。
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('aio-plugin-bindings',0))")
        .execute(&mut *target)
        .await?;
    let schema: String = sqlx::query_scalar("SELECT schema_name FROM aio_plugin_host.database_bindings WHERE source_id=$1 AND tenant_id=$2")
        .bind(source.to_string()).bind(tenant).fetch_optional(&mut *target).await?
        .context("当前租户尚未安装字典整包，不能推断目标 schema")?;
    ensure!(
        schema.starts_with("p_")
            && schema.len() <= 63
            && schema
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_'),
        "隔离 schema 无效"
    );
    sqlx::query(&format!("LOCK TABLE {schema}.dictionary_types, {schema}.dictionary_items IN SHARE ROW EXCLUSIVE MODE"))
        .execute(&mut *target).await?;
    let mut counts = Vec::new();
    for table in ["dictionary_types", "dictionary_items"] {
        let rows: Value = sqlx::query_scalar(&format!("SELECT coalesce(json_agg(t ORDER BY id), '[]'::json) FROM public.{table} t WHERE tenant_id=$1"))
            .bind(tenant).fetch_one(&mut *old).await?;
        let count = rows.as_array().context("旧字典快照无效")?.len();
        sqlx::query(&format!("INSERT INTO {schema}.{table} SELECT * FROM json_populate_recordset(NULL::{schema}.{table}, $1::json) ON CONFLICT DO NOTHING"))
            .bind(&rows).execute(&mut *target).await.context("复制字典数据失败，目标事务将回滚")?;
        let conflicts: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM json_populate_recordset(NULL::{schema}.{table}, $1::json) s LEFT JOIN {schema}.{table} t USING(tenant_id,id) WHERE to_jsonb(s) IS DISTINCT FROM to_jsonb(t)"))
            .bind(&rows).fetch_one(&mut *target).await?;
        ensure!(
            conflicts == 0,
            "{table} 有 {conflicts} 条目标数据不同，拒绝覆盖，复制事务将回滚"
        );
        counts.push(count);
    }
    target.commit().await?;
    old.rollback().await?;
    Ok(MigrationReport {
        types: counts[0],
        items: counts[1],
    })
}
