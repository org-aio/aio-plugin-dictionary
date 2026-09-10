#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DictionaryView {
    pub types: Vec<DictionaryType>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DictionaryType {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub items: Vec<DictionaryItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DictionaryItem {
    pub id: String,
    pub type_id: String,
    pub value: String,
    pub label: String,
    pub sort_order: i32,
    pub is_default: bool,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateDictionaryTypeRequest {
    pub code: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateDictionaryTypeRequest {
    pub code: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateDictionaryItemRequest {
    pub type_id: String,
    pub value: String,
    pub label: String,
    pub sort_order: i32,
    pub is_default: bool,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateDictionaryItemRequest {
    pub type_id: String,
    pub value: String,
    pub label: String,
    pub sort_order: i32,
    pub is_default: bool,
    pub enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DictionaryResponse<T> {
    pub data: T,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DictionaryErrorResponse {
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_models_do_not_expose_tenant_identity() {
        let value = serde_json::to_value(CreateDictionaryTypeRequest {
            code: "order-status".to_owned(),
            name: "订单状态".to_owned(),
            description: String::new(),
            enabled: true,
        })
        .expect("请求应可序列化");

        assert!(value.get("tenant_id").is_none());
    }
}
