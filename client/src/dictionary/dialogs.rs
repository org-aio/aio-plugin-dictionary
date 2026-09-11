use super::http;
use aio_plugin_dictionary_model::{
    CreateDictionaryItemRequest, CreateDictionaryTypeRequest, DictionaryItem, DictionaryType,
    UpdateDictionaryItemRequest, UpdateDictionaryTypeRequest,
};
use az_ui_components::{
    admin::StatusMessage,
    button::{Button, ButtonVariant},
    checkbox::{Checkbox, CheckboxState},
    dialog::{Dialog, DialogDescription, DialogTitle},
    input::Input,
    textarea::Textarea,
};
use dioxus::prelude::*;

#[component]
pub(super) fn TypeEditorDialog(
    value: Option<DictionaryType>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let id = value.as_ref().map(|item| item.id.clone());
    let mut draft = use_signal(|| UpdateDictionaryTypeRequest {
        code: value
            .as_ref()
            .map(|item| item.code.clone())
            .unwrap_or_default(),
        name: value
            .as_ref()
            .map(|item| item.name.clone())
            .unwrap_or_default(),
        description: value
            .as_ref()
            .map(|item| item.description.clone())
            .unwrap_or_default(),
        enabled: value.as_ref().is_none_or(|item| item.enabled),
    });
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        Dialog { open: true, on_open_change: move |open: bool| if !open && !busy() { on_close.call(()) },
            DialogTitle { if value.is_some() { "编辑字典类型" } else { "新建字典类型" } }
            DialogDescription { "类型编码在当前租户内唯一。" }
            form { class: "admin-form", onsubmit: move |event: FormEvent| {
                event.prevent_default(); if busy() { return; }
                busy.set(true); error.set(None);
                let id = id.clone(); let payload = draft();
                spawn(async move {
                    let result = match id {
                        Some(id) => http::update_type(&id, payload).await,
                        None => http::create_type(CreateDictionaryTypeRequest { code: payload.code, name: payload.name, description: payload.description, enabled: payload.enabled }).await,
                    };
                    busy.set(false);
                    match result { Ok(_) => on_saved.call(()), Err(message) => error.set(Some(message)) }
                });
            },
                fieldset { class: "admin-form", disabled: busy(),
                    label { class: "admin-field", span { "类型编码" } Input { aria_label: "字典类型编码", value: draft().code, required: true, maxlength: "96", oninput: move |event: FormEvent| draft.write().code = event.value() } }
                    label { class: "admin-field", span { "类型名称" } Input { aria_label: "字典类型名称", value: draft().name, required: true, oninput: move |event: FormEvent| draft.write().name = event.value() } }
                    label { class: "admin-field", span { "说明" } Textarea { aria_label: "字典类型说明", value: draft().description, oninput: move |event: FormEvent| draft.write().description = event.value() } }
                    label { class: "admin-actions", Checkbox { aria_label: "启用类型", checked: Some(if draft().enabled { CheckboxState::Checked } else { CheckboxState::Unchecked }), on_checked_change: move |state| draft.write().enabled = bool::from(state) } "启用类型" }
                }
                if let Some(message) = error() { StatusMessage { error: true, message } }
                DialogActions { busy: busy(), on_close }
            }
        }
    }
}

#[component]
pub(super) fn ItemEditorDialog(
    type_id: String,
    value: Option<DictionaryItem>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
) -> Element {
    let id = value.as_ref().map(|item| item.id.clone());
    let mut draft = use_signal(|| UpdateDictionaryItemRequest {
        type_id,
        value: value
            .as_ref()
            .map(|item| item.value.clone())
            .unwrap_or_default(),
        label: value
            .as_ref()
            .map(|item| item.label.clone())
            .unwrap_or_default(),
        sort_order: value.as_ref().map_or(0, |item| item.sort_order),
        is_default: value.as_ref().is_some_and(|item| item.is_default),
        enabled: value.as_ref().is_none_or(|item| item.enabled),
    });
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    rsx! {
        Dialog { open: true, on_open_change: move |open: bool| if !open && !busy() { on_close.call(()) },
            DialogTitle { if value.is_some() { "编辑字典项" } else { "新建字典项" } }
            DialogDescription { "每个类型只有一个默认项，字典值不能重复。" }
            form { class: "admin-form", onsubmit: move |event: FormEvent| {
                event.prevent_default(); if busy() { return; } busy.set(true); error.set(None);
                let id = id.clone(); let payload = draft();
                spawn(async move {
                    let result = match id {
                        Some(id) => http::update_item(&id, payload).await,
                        None => http::create_item(CreateDictionaryItemRequest { type_id: payload.type_id, value: payload.value, label: payload.label, sort_order: payload.sort_order, enabled: payload.enabled, is_default: payload.is_default }).await,
                    };
                    busy.set(false);
                    match result { Ok(_) => on_saved.call(()), Err(message) => error.set(Some(message)) }
                });
            },
                fieldset { class: "admin-form", disabled: busy(),
                    label { class: "admin-field", span { "字典值" } Input { aria_label: "字典值", value: draft().value, required: true, oninput: move |event: FormEvent| draft.write().value = event.value() } }
                    label { class: "admin-field", span { "显示标签" } Input { aria_label: "字典项标签", value: draft().label, required: true, oninput: move |event: FormEvent| draft.write().label = event.value() } }
                    label { class: "admin-field", span { "排序" } Input { aria_label: "字典项排序", r#type: "number", required: true, value: draft().sort_order.to_string(), oninput: move |event: FormEvent| { if let Ok(value) = event.value().parse() { draft.write().sort_order = value; } } } }
                    label { class: "admin-actions", Checkbox { aria_label: "默认项", checked: Some(if draft().is_default { CheckboxState::Checked } else { CheckboxState::Unchecked }), on_checked_change: move |state| draft.write().is_default = bool::from(state) } "默认项" }
                    label { class: "admin-actions", Checkbox { aria_label: "启用字典项", checked: Some(if draft().enabled { CheckboxState::Checked } else { CheckboxState::Unchecked }), on_checked_change: move |state| draft.write().enabled = bool::from(state) } "启用字典项" }
                }
                if let Some(message) = error() { StatusMessage { error: true, message } }
                DialogActions { busy: busy(), on_close }
            }
        }
    }
}

#[component]
fn DialogActions(busy: bool, on_close: Callback<()>) -> Element {
    rsx! { footer { class: "admin-form-footer",
        Button { r#type: "button", variant: ButtonVariant::Outline, disabled: busy, onclick: move |_| on_close.call(()), "取消" }
        Button { r#type: "submit", disabled: busy, if busy { "正在保存" } else { "保存" } }
    } }
}
