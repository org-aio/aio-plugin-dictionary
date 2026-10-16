//! 真实进程、受限 PostgreSQL 和旧数据复制；只允许本机专用测试库。
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use sqlx::{
    ConnectOptions,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{process::Stdio, time::Duration};

#[tokio::test]
#[ignore = "需要本机 dictionary_test PostgreSQL"]
async fn hosted_process_preserves_identity_and_migration_is_atomic() -> Result<()> {
    let url = std::env::var("AIO_TEST_DATABASE_URL")?;
    let options: PgConnectOptions = url.parse()?;
    ensure!(
        options.get_host() == "127.0.0.1" && options.get_database() == Some("dictionary_test"),
        "只接受本机专用库"
    );
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await?;
    let id = uuid::Uuid::new_v4();
    let tenant = format!("tenant-{id}");
    let schema = format!("p_{}", id.simple());
    let role = format!("r_{schema}");
    let password = uuid::Uuid::new_v4().simple().to_string();
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS component_sources(id UUID PRIMARY KEY,git TEXT UNIQUE NOT NULL); CREATE SCHEMA IF NOT EXISTS aio_plugin_host; CREATE TABLE IF NOT EXISTS aio_plugin_host.database_bindings(source_id TEXT,tenant_id TEXT,schema_name TEXT,PRIMARY KEY(source_id,tenant_id))").execute(&pool).await?;
    let source: uuid::Uuid = sqlx::query_scalar("INSERT INTO component_sources(id,git) VALUES($1,'https://github.com/org-aio/aio-plugin-dictionary.git') ON CONFLICT(git) DO UPDATE SET git=EXCLUDED.git RETURNING id")
        .bind(id).fetch_one(&pool).await?;
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&pool)
        .await?;
    let mut tx = pool.begin().await?;
    sqlx::query(&format!("SET LOCAL search_path={schema}"))
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(include_str!("../migrations/001_dictionary.sql"))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    sqlx::query(&format!("CREATE ROLE {role} LOGIN PASSWORD '{password}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOBYPASSRLS" )).execute(&pool).await?;
    for sql in [
        format!("GRANT USAGE ON SCHEMA {schema} TO {role}"),
        format!("ALTER ROLE {role} SET search_path={schema}"),
        format!("GRANT SELECT,INSERT,UPDATE,DELETE ON ALL TABLES IN SCHEMA {schema} TO {role}"),
    ] {
        sqlx::query(&sql).execute(&pool).await?;
    }
    sqlx::query("INSERT INTO aio_plugin_host.database_bindings VALUES($1,$2,$3)")
        .bind(source.to_string())
        .bind(&tenant)
        .bind(&schema)
        .execute(&pool)
        .await?;
    sqlx::raw_sql(include_str!("../migrations/001_dictionary.sql"))
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO dictionary_types(tenant_id,id,code,name) VALUES($1,'legacy','status','状态'),('other-tenant','legacy','status','其他租户') ON CONFLICT DO NOTHING").bind(&tenant).execute(&pool).await?;
    sqlx::query("INSERT INTO dictionary_items(tenant_id,id,type_id,value,label,is_default) VALUES($1,'legacy-item','legacy','open','开启',true)").bind(&tenant).execute(&pool).await?;
    let copied =
        aio_plugin_dictionary_server::migration::copy_legacy(&pool, &pool, &tenant).await?;
    assert_eq!((copied.types, copied.items), (1, 1));
    assert_eq!(
        aio_plugin_dictionary_server::migration::copy_legacy(&pool, &pool, &tenant).await?,
        copied
    );
    // 同一批次中的新行不能在另一条冲突被拒绝后残留。
    sqlx::query(
        "INSERT INTO dictionary_types(tenant_id,id,code,name) VALUES($1,'new','new','新类型')",
    )
    .bind(&tenant)
    .execute(&pool)
    .await?;
    sqlx::query(&format!(
        "UPDATE {schema}.dictionary_items SET label='目标已编辑'"
    ))
    .execute(&pool)
    .await?;
    assert!(
        aio_plugin_dictionary_server::migration::copy_legacy(&pool, &pool, &tenant)
            .await
            .is_err()
    );
    let rows: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {schema}.dictionary_types"))
        .fetch_one(&pool)
        .await?;
    assert_eq!(rows, 1);

    let directory = tempfile::tempdir()?;
    let config = directory.path().join("config.json");
    let socket = directory.path().join("service.sock");
    let token = uuid::Uuid::new_v4().simple().to_string();
    let scoped = options
        .username(&role)
        .password(&password)
        .to_url_lossy()
        .to_string();
    std::fs::write(
        &config,
        serde_json::to_vec(
            &json!({"abi_version":2,"tenant_id":tenant,"database_url":scoped,"encryption_key":null,"ingress_token":token,"broker_socket":"/unused/broker.sock","endpoints":[],"services":[]}),
        )?,
    )?;
    let mut child =
        tokio::process::Command::new(env!("CARGO_BIN_EXE_aio-plugin-dictionary-server"))
            .env_clear()
            .env("AIO_PLUGIN_CONFIG", &config)
            .env("AIO_PLUGIN_SOCKET", &socket)
            .stdout(Stdio::null())
            .kill_on_drop(true)
            .spawn()?;
    let client = reqwest::Client::builder()
        .unix_socket(socket.as_path())
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()?;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if client
                .get("http://localhost/health")
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                return anyhow::Ok(());
            }
            ensure!(child.try_wait()?.is_none(), "字典进程启动失败");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await??;
    let call = |method, path: &str| {
        client
            .request(method, format!("http://localhost{path}"))
            .header("x-aio-token", &token)
            .header("x-aio-tenant-id", &tenant)
            .header("x-aio-user-id", "admin")
            .header("x-aio-context", "trusted-context")
    };
    let response: Value = call(reqwest::Method::GET, "/api/dictionaries")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        response["data"]["types"]
            .as_array()
            .context("缺少类型")?
            .len(),
        1
    );
    assert_eq!(
        response["data"]["types"][0]["items"][0]["label"],
        "目标已编辑"
    );
    let created: Value = call(reqwest::Method::POST, "/api/dictionaries/types")
        .json(&json!({"code":"process","name":"进程创建","description":"","enabled":true}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(created["data"]["id"].is_string());
    assert_eq!(
        client
            .get("http://localhost/api/dictionaries")
            .header("x-aio-tenant-id", &tenant)
            .send()
            .await?
            .status(),
        401
    );
    assert_eq!(
        client
            .get("http://localhost/api/dictionaries")
            .header("x-aio-token", &token)
            .header("x-aio-user-id", "admin")
            .header("x-aio-context", "trusted-context")
            .header("x-aio-tenant-id", "other-tenant")
            .send()
            .await?
            .status(),
        403
    );
    let process_id = child.id().context("缺少进程")?;
    ensure!(
        tokio::process::Command::new("kill")
            .args(["-TERM", &process_id.to_string()])
            .status()
            .await?
            .success(),
        "发送停止信号失败"
    );
    ensure!(
        tokio::time::timeout(Duration::from_secs(10), child.wait())
            .await??
            .success(),
        "进程未正常排空退出"
    );
    let active: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE usename=$1")
        .bind(&role)
        .fetch_one(&pool)
        .await?;
    assert_eq!(active, 0);
    pool.close().await;
    Ok(())
}
