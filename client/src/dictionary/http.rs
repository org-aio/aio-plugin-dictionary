use aio_plugin_dictionary_model::{
    CreateDictionaryItemRequest, CreateDictionaryTypeRequest, DictionaryErrorResponse,
    DictionaryItem, DictionaryResponse, DictionaryType, DictionaryView,
    UpdateDictionaryItemRequest, UpdateDictionaryTypeRequest,
};
use serde::{Serialize, de::DeserializeOwned};

pub async fn load() -> Result<DictionaryView, String> {
    request_json::<DictionaryView, ()>("GET", "/api/dictionaries", None).await
}

pub async fn create_type(request: CreateDictionaryTypeRequest) -> Result<DictionaryType, String> {
    request_json("POST", "/api/dictionaries/types", Some(&request)).await
}

pub async fn update_type(
    type_id: &str,
    request: UpdateDictionaryTypeRequest,
) -> Result<DictionaryType, String> {
    request_json(
        "PUT",
        &format!("/api/dictionaries/types/{type_id}"),
        Some(&request),
    )
    .await
}

pub async fn delete_type(type_id: &str) -> Result<(), String> {
    request_empty("DELETE", &format!("/api/dictionaries/types/{type_id}")).await
}

pub async fn create_item(request: CreateDictionaryItemRequest) -> Result<DictionaryItem, String> {
    request_json("POST", "/api/dictionaries/items", Some(&request)).await
}

pub async fn update_item(
    item_id: &str,
    request: UpdateDictionaryItemRequest,
) -> Result<DictionaryItem, String> {
    request_json(
        "PUT",
        &format!("/api/dictionaries/items/{item_id}"),
        Some(&request),
    )
    .await
}

pub async fn delete_item(item_id: &str) -> Result<(), String> {
    request_empty("DELETE", &format!("/api/dictionaries/items/{item_id}")).await
}

async fn request_json<T, B>(method: &str, path: &str, body: Option<&B>) -> Result<T, String>
where
    T: DeserializeOwned,
    B: Serialize + ?Sized,
{
    let value = request(method, path, body).await?;
    serde_json::from_value::<DictionaryResponse<T>>(value)
        .map(|response| response.data)
        .map_err(|error| error.to_string())
}

async fn request_empty(method: &str, path: &str) -> Result<(), String> {
    request::<()>(method, path, None).await?;
    Ok(())
}

// 复用宿主注入的 v2 SDK；数据通过消息传递，不把用户输入拼成 JavaScript。
async fn request<B: Serialize + ?Sized>(
    method: &str,
    path: &str,
    body: Option<&B>,
) -> Result<serde_json::Value, String> {
    let mut bridge = dioxus::document::eval(
        r#"
        const input = await dioxus.recv();
        try {
            const value = await window.aioPlugin.json(input.method, input.path, input.body ?? undefined);
            dioxus.send({ value });
        } catch (error) {
            dioxus.send({ error: String(error.message ?? error) });
        }
    "#,
    );
    bridge
        .send(serde_json::json!({ "method": method, "path": path, "body": body }))
        .map_err(|error| error.to_string())?;
    let response: serde_json::Value = bridge.recv().await.map_err(|error| error.to_string())?;
    if let Some(error) = response.get("error").and_then(serde_json::Value::as_str) {
        return Err(decode_error(error));
    }
    response
        .get("value")
        .cloned()
        .ok_or_else(|| "宿主未返回字典结果".into())
}

fn decode_error(body: &str) -> String {
    serde_json::from_str::<DictionaryErrorResponse>(body)
        .map(|response| response.error)
        .unwrap_or_else(|_| body.to_owned())
}
