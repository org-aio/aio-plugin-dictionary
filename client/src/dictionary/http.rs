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

#[cfg(target_arch = "wasm32")]
async fn request_json<T, B>(method: &str, path: &str, body: Option<&B>) -> Result<T, String>
where
    T: DeserializeOwned,
    B: Serialize + ?Sized,
{
    let request = match method {
        "GET" => gloo_net::http::Request::get(path),
        "POST" => gloo_net::http::Request::post(path),
        "PUT" => gloo_net::http::Request::put(path),
        _ => return Err(format!("不支持的请求方法：{method}")),
    };
    let request = match body {
        Some(body) => request.json(body).map_err(|error| error.to_string())?,
        None => request.build().map_err(|error| error.to_string())?,
    };
    let response = request.send().await.map_err(|error| error.to_string())?;
    if !response.ok() {
        return Err(decode_error(response.text().await.unwrap_or_default()));
    }
    response
        .json::<DictionaryResponse<T>>()
        .await
        .map(|response| response.data)
        .map_err(|error| error.to_string())
}

#[cfg(target_arch = "wasm32")]
async fn request_empty(method: &str, path: &str) -> Result<(), String> {
    if method != "DELETE" {
        return Err(format!("不支持的请求方法：{method}"));
    }
    let response = gloo_net::http::Request::delete(path)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if response.ok() {
        Ok(())
    } else {
        Err(decode_error(response.text().await.unwrap_or_default()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn request_json<T, B>(method: &str, path: &str, body: Option<&B>) -> Result<T, String>
where
    T: DeserializeOwned,
    B: Serialize + ?Sized,
{
    let client = reqwest::Client::new();
    let method =
        reqwest::Method::from_bytes(method.as_bytes()).map_err(|error| error.to_string())?;
    let mut request = client.request(method, endpoint(path));
    if let Ok(cookie) = std::env::var("AIO_SESSION_COOKIE") {
        request = request.header(reqwest::header::COOKIE, cookie);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(decode_error(String::from_utf8_lossy(&bytes).into_owned()));
    }
    serde_json::from_slice::<DictionaryResponse<T>>(&bytes)
        .map(|response| response.data)
        .map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn request_empty(method: &str, path: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let method =
        reqwest::Method::from_bytes(method.as_bytes()).map_err(|error| error.to_string())?;
    let mut request = client.request(method, endpoint(path));
    if let Ok(cookie) = std::env::var("AIO_SESSION_COOKIE") {
        request = request.header(reqwest::header::COOKIE, cookie);
    }
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = response.text().await.unwrap_or_default();
        Err(decode_error(body))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn endpoint(path: &str) -> String {
    let base =
        std::env::var("AIO_API_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:4173".to_owned());
    format!("{}{path}", base.trim_end_matches('/'))
}

fn decode_error(body: String) -> String {
    serde_json::from_str::<DictionaryErrorResponse>(&body)
        .map(|response| response.error)
        .unwrap_or(body)
}
