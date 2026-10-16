use std::sync::Arc;

use aio_plugin_dictionary_model::{
    CreateDictionaryItemRequest, CreateDictionaryTypeRequest, DictionaryErrorResponse,
    DictionaryItem, DictionaryResponse, DictionaryType, DictionaryView,
    UpdateDictionaryItemRequest, UpdateDictionaryTypeRequest,
};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use az_plugin_process::Caller;

use super::service::DictionaryService;

#[dill::component]
#[dill::scope(dill::Singleton)]
pub struct DictionaryController {
    service: Arc<dyn DictionaryService>,
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
    Extension(session): Extension<Caller>,
) -> Result<Json<DictionaryResponse<DictionaryView>>, DictionaryHttpError> {
    Ok(Json(DictionaryResponse {
        data: controller.service.view(&session.tenant_id).await?,
    }))
}

async fn create_type(
    State(controller): State<Arc<DictionaryController>>,
    Extension(session): Extension<Caller>,
    Json(request): Json<CreateDictionaryTypeRequest>,
) -> Result<(StatusCode, Json<DictionaryResponse<DictionaryType>>), DictionaryHttpError> {
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
    Extension(session): Extension<Caller>,
    Path(type_id): Path<String>,
    Json(request): Json<UpdateDictionaryTypeRequest>,
) -> Result<Json<DictionaryResponse<DictionaryType>>, DictionaryHttpError> {
    let value = controller
        .service
        .update_type(&session.tenant_id, &type_id, request)
        .await?;
    Ok(Json(DictionaryResponse { data: value }))
}

async fn delete_type(
    State(controller): State<Arc<DictionaryController>>,
    Extension(session): Extension<Caller>,
    Path(type_id): Path<String>,
) -> Result<StatusCode, DictionaryHttpError> {
    controller
        .service
        .delete_type(&session.tenant_id, &type_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_item(
    State(controller): State<Arc<DictionaryController>>,
    Extension(session): Extension<Caller>,
    Json(request): Json<CreateDictionaryItemRequest>,
) -> Result<(StatusCode, Json<DictionaryResponse<DictionaryItem>>), DictionaryHttpError> {
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
    Extension(session): Extension<Caller>,
    Path(item_id): Path<String>,
    Json(request): Json<UpdateDictionaryItemRequest>,
) -> Result<Json<DictionaryResponse<DictionaryItem>>, DictionaryHttpError> {
    let value = controller
        .service
        .update_item(&session.tenant_id, &item_id, request)
        .await?;
    Ok(Json(DictionaryResponse { data: value }))
}

async fn delete_item(
    State(controller): State<Arc<DictionaryController>>,
    Extension(session): Extension<Caller>,
    Path(item_id): Path<String>,
) -> Result<StatusCode, DictionaryHttpError> {
    controller
        .service
        .delete_item(&session.tenant_id, &item_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
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
