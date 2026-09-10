use std::sync::Arc;

use aio_plugin_dictionary_model::{
    CreateDictionaryItemRequest, CreateDictionaryTypeRequest, DictionaryErrorResponse,
    DictionaryItem, DictionaryResponse, DictionaryType, DictionaryView,
    UpdateDictionaryItemRequest, UpdateDictionaryTypeRequest,
};
use aio_plugin_identity_server::{IdentityService, SessionContext};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};

use super::service::DictionaryService;

#[dill::component]
#[dill::scope(dill::Singleton)]
pub(super) struct DictionaryController {
    service: Arc<dyn DictionaryService>,
    identity: Arc<IdentityService>,
}

pub(super) fn router(controller: Arc<DictionaryController>) -> Router {
    Router::new()
        .route("/api/plugins/dictionary/health", get(health))
        .route("/api/dictionaries", get(view))
        .route("/api/dictionaries/types", post(create_type))
        .route(
            "/api/dictionaries/types/{type_id}",
            put(update_type).delete(delete_type),
        )
        .route("/api/dictionaries/items", post(create_item))
        .route(
            "/api/dictionaries/items/{item_id}",
            put(update_item).delete(delete_item),
        )
        .with_state(controller)
}

async fn health() -> &'static str {
    "ok"
}

async fn view(
    State(controller): State<Arc<DictionaryController>>,
    headers: HeaderMap,
) -> Result<Json<DictionaryResponse<DictionaryView>>, DictionaryHttpError> {
    let session = controller.authenticate(&headers).await?;
    Ok(Json(DictionaryResponse {
        data: controller.service.view(&session.tenant_id).await?,
    }))
}

async fn create_type(
    State(controller): State<Arc<DictionaryController>>,
    headers: HeaderMap,
    Json(request): Json<CreateDictionaryTypeRequest>,
) -> Result<(StatusCode, Json<DictionaryResponse<DictionaryType>>), DictionaryHttpError> {
    let session = controller.authenticate(&headers).await?;
    let value = controller
        .service
        .create_type(&session.tenant_id, request)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(DictionaryResponse { data: value }),
    ))
}

async fn update_type(
    State(controller): State<Arc<DictionaryController>>,
    headers: HeaderMap,
    Path(type_id): Path<String>,
    Json(request): Json<UpdateDictionaryTypeRequest>,
) -> Result<Json<DictionaryResponse<DictionaryType>>, DictionaryHttpError> {
    let session = controller.authenticate(&headers).await?;
    let value = controller
        .service
        .update_type(&session.tenant_id, &type_id, request)
        .await?;
    Ok(Json(DictionaryResponse { data: value }))
}

async fn delete_type(
    State(controller): State<Arc<DictionaryController>>,
    headers: HeaderMap,
    Path(type_id): Path<String>,
) -> Result<StatusCode, DictionaryHttpError> {
    let session = controller.authenticate(&headers).await?;
    controller
        .service
        .delete_type(&session.tenant_id, &type_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_item(
    State(controller): State<Arc<DictionaryController>>,
    headers: HeaderMap,
    Json(request): Json<CreateDictionaryItemRequest>,
) -> Result<(StatusCode, Json<DictionaryResponse<DictionaryItem>>), DictionaryHttpError> {
    let session = controller.authenticate(&headers).await?;
    let value = controller
        .service
        .create_item(&session.tenant_id, request)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(DictionaryResponse { data: value }),
    ))
}

async fn update_item(
    State(controller): State<Arc<DictionaryController>>,
    headers: HeaderMap,
    Path(item_id): Path<String>,
    Json(request): Json<UpdateDictionaryItemRequest>,
) -> Result<Json<DictionaryResponse<DictionaryItem>>, DictionaryHttpError> {
    let session = controller.authenticate(&headers).await?;
    let value = controller
        .service
        .update_item(&session.tenant_id, &item_id, request)
        .await?;
    Ok(Json(DictionaryResponse { data: value }))
}

async fn delete_item(
    State(controller): State<Arc<DictionaryController>>,
    headers: HeaderMap,
    Path(item_id): Path<String>,
) -> Result<StatusCode, DictionaryHttpError> {
    let session = controller.authenticate(&headers).await?;
    controller
        .service
        .delete_item(&session.tenant_id, &item_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

impl DictionaryController {
    async fn authenticate(
        &self,
        headers: &HeaderMap,
    ) -> Result<SessionContext, DictionaryHttpError> {
        let session = self.identity.authenticate(headers).await?.ok_or_else(|| {
            DictionaryHttpError::new(StatusCode::UNAUTHORIZED, "会话无效或已过期")
        })?;
        if !session
            .permissions
            .iter()
            .any(|permission| permission == "dictionary:manage")
        {
            return Err(DictionaryHttpError::new(
                StatusCode::FORBIDDEN,
                "当前角色没有字典管理权限",
            ));
        }
        Ok(session)
    }
}

struct DictionaryHttpError {
    status: StatusCode,
    message: String,
}

impl DictionaryHttpError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl<E> From<E> for DictionaryHttpError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self::new(StatusCode::BAD_REQUEST, format!("{:#}", value.into()))
    }
}

impl IntoResponse for DictionaryHttpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(DictionaryErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}
