use aio_plugin_dictionary_model::{
    CreateDictionaryItemRequest, CreateDictionaryTypeRequest, DictionaryItem, DictionaryType,
    UpdateDictionaryItemRequest, UpdateDictionaryTypeRequest,
};
use az_ui_components::{
    button::{Button, ButtonVariant},
    checkbox::{Checkbox, CheckboxState},
    dialog::{Dialog, DialogDescription, DialogTitle},
    input::Input,
    textarea::Textarea,
};
use dioxus::prelude::*;

use super::http;

#[allow(non_snake_case)]
#[component]
pub(super) fn TypeEditorDialog(
    value: Option<DictionaryType>,
    on_close: EventHandler<()>,
    on_saved: EventHandler<()>,
) -> Element {
    let editing = value.is_some();
    let id = value.as_ref().map(|item| item.id.clone());
    let mut code = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.code.clone())
            .unwrap_or_default()
    });
    let mut name = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.name.clone())
            .unwrap_or_default()
    });
    let mut description = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.description.clone())
            .unwrap_or_default()
    });
    let mut enabled = use_signal(|| value.as_ref().is_none_or(|item| item.enabled));
    let mut error = use_signal(|| None::<String>);
    rsx! {
        Dialog {
            open: true,
            on_open_change: move |open: bool| if !open { on_close.call(()) },
            form {
                class: "grid gap-3",
                onsubmit: move |event| {
                    event.prevent_default();
                    let id = id.clone();
                    spawn(async move {
                        let result = if let Some(id) = id {
                            http::update_type(&id, UpdateDictionaryTypeRequest {
                                code: code(),
                                name: name(),
                                description: description(),
                                enabled: enabled(),
                            }).await.map(|_| ())
                        } else {
                            http::create_type(CreateDictionaryTypeRequest {
                                code: code(),
                                name: name(),
                                description: description(),
                                enabled: enabled(),
                            }).await.map(|_| ())
                        };
                        match result {
                            Ok(()) => on_saved.call(()),
                            Err(message) => error.set(Some(message)),
                        }
                    });
                },
                DialogTitle { if editing { "编辑字典类型" } else { "新建字典类型" } }
                DialogDescription { "类型编码在当前租户内唯一，保存后立即用于字典项归类。" }
                label { r#for: "dictionary-type-code", "类型编码" }
                Input {
                    id: "dictionary-type-code",
                    aria_label: "字典类型编码",
                    value: code(),
                    required: true,
                    oninput: move |event: FormEvent| code.set(event.value()),
                }
                label { r#for: "dictionary-type-name", "类型名称" }
                Input {
                    id: "dictionary-type-name",
                    aria_label: "字典类型名称",
                    value: name(),
                    required: true,
                    oninput: move |event: FormEvent| name.set(event.value()),
                }
                label { r#for: "dictionary-type-description", "说明" }
                Textarea {
                    id: "dictionary-type-description",
                    aria_label: "字典类型说明",
                    value: description(),
                    oninput: move |event: FormEvent| description.set(event.value()),
                }
                label { class: "flex items-center gap-2",
                    Checkbox {
                        checked: Some(if enabled() { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                        on_checked_change: move |state| enabled.set(bool::from(state)),
                    }
                    "启用类型"
                }
                if let Some(message) = error() {
                    p { role: "alert", "{message}" }
                }
                DialogActions { on_close }
            }
        }
    }
}

#[allow(non_snake_case)]
#[component]
pub(super) fn ItemEditorDialog(
    type_id: String,
    value: Option<DictionaryItem>,
    on_close: EventHandler<()>,
    on_saved: EventHandler<()>,
) -> Element {
    let editing = value.is_some();
    let id = value.as_ref().map(|item| item.id.clone());
    let mut item_value = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.value.clone())
            .unwrap_or_default()
    });
    let mut label = use_signal(|| {
        value
            .as_ref()
            .map(|item| item.label.clone())
            .unwrap_or_default()
    });
    let mut sort_order = use_signal(|| value.as_ref().map_or(0, |item| item.sort_order));
    let mut is_default = use_signal(|| value.as_ref().is_some_and(|item| item.is_default));
    let mut enabled = use_signal(|| value.as_ref().is_none_or(|item| item.enabled));
    let mut error = use_signal(|| None::<String>);
    rsx! {
        Dialog {
            open: true,
            on_open_change: move |open: bool| if !open { on_close.call(()) },
            form {
                class: "grid gap-3",
                onsubmit: move |event| {
                    event.prevent_default();
                    let id = id.clone();
                    let type_id = type_id.clone();
                    spawn(async move {
                        let result = if let Some(id) = id {
                            http::update_item(&id, UpdateDictionaryItemRequest {
                                type_id,
                                value: item_value(),
                                label: label(),
                                sort_order: sort_order(),
                                is_default: is_default(),
                                enabled: enabled(),
                            }).await.map(|_| ())
                        } else {
                            http::create_item(CreateDictionaryItemRequest {
                                type_id,
                                value: item_value(),
                                label: label(),
                                sort_order: sort_order(),
                                is_default: is_default(),
                                enabled: enabled(),
                            }).await.map(|_| ())
                        };
                        match result {
                            Ok(()) => on_saved.call(()),
                            Err(message) => error.set(Some(message)),
                        }
                    });
                },
                DialogTitle { if editing { "编辑字典项" } else { "新建字典项" } }
                DialogDescription { "字典值在当前类型内唯一；设为默认时会自动取消原默认项。" }
                label { r#for: "dictionary-item-value", "字典值" }
                Input {
                    id: "dictionary-item-value",
                    aria_label: "字典值",
                    value: item_value(),
                    required: true,
                    oninput: move |event: FormEvent| item_value.set(event.value()),
                }
                label { r#for: "dictionary-item-label", "显示标签" }
                Input {
                    id: "dictionary-item-label",
                    aria_label: "字典项标签",
                    value: label(),
                    required: true,
                    oninput: move |event: FormEvent| label.set(event.value()),
                }
                label { r#for: "dictionary-item-order", "排序" }
                Input {
                    id: "dictionary-item-order",
                    r#type: "number",
                    aria_label: "字典项排序",
                    value: sort_order().to_string(),
                    oninput: move |event: FormEvent| {
                        if let Ok(value) = event.value().parse() {
                            sort_order.set(value);
                        }
                    },
                }
                label { class: "flex items-center gap-2",
                    Checkbox {
                        checked: Some(if is_default() { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                        on_checked_change: move |state| is_default.set(bool::from(state)),
                    }
                    "默认项"
                }
                label { class: "flex items-center gap-2",
                    Checkbox {
                        checked: Some(if enabled() { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                        on_checked_change: move |state| enabled.set(bool::from(state)),
                    }
                    "启用字典项"
                }
                if let Some(message) = error() {
                    p { role: "alert", "{message}" }
                }
                DialogActions { on_close }
            }
        }
    }
}

#[allow(non_snake_case)]
#[component]
pub(super) fn DeleteTypeDialog(
    value: DictionaryType,
    on_close: EventHandler<()>,
    on_deleted: EventHandler<()>,
) -> Element {
    let mut error = use_signal(|| None::<String>);
    rsx! {
        Dialog {
            open: true,
            on_open_change: move |open: bool| if !open { on_close.call(()) },
            DialogTitle { "删除字典类型" }
            DialogDescription {
                "将删除“{value.name}”及其全部 {value.items.len()} 个字典项，此操作不可撤销。"
            }
            if let Some(message) = error() {
                p { role: "alert", "{message}" }
            }
            footer { class: "flex justify-end gap-2",
                Button {
                    r#type: "button",
                    variant: ButtonVariant::Ghost,
                    onclick: move |_| on_close.call(()),
                    "取消"
                }
                Button {
                    r#type: "button",
                    variant: ButtonVariant::Destructive,
                    onclick: move |_| {
                        let id = value.id.clone();
                        spawn(async move {
                            match http::delete_type(&id).await {
                                Ok(()) => on_deleted.call(()),
                                Err(message) => error.set(Some(message)),
                            }
                        });
                    },
                    "确认删除"
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[component]
pub(super) fn DeleteItemDialog(
    value: DictionaryItem,
    on_close: EventHandler<()>,
    on_deleted: EventHandler<()>,
) -> Element {
    let mut error = use_signal(|| None::<String>);
    rsx! {
        Dialog {
            open: true,
            on_open_change: move |open: bool| if !open { on_close.call(()) },
            DialogTitle { "删除字典项" }
            DialogDescription { "将删除“{value.label}（{value.value}）”，此操作不可撤销。" }
            if let Some(message) = error() {
                p { role: "alert", "{message}" }
            }
            footer { class: "flex justify-end gap-2",
                Button {
                    r#type: "button",
                    variant: ButtonVariant::Ghost,
                    onclick: move |_| on_close.call(()),
                    "取消"
                }
                Button {
                    r#type: "button",
                    variant: ButtonVariant::Destructive,
                    onclick: move |_| {
                        let id = value.id.clone();
                        spawn(async move {
                            match http::delete_item(&id).await {
                                Ok(()) => on_deleted.call(()),
                                Err(message) => error.set(Some(message)),
                            }
                        });
                    },
                    "确认删除"
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[component]
fn DialogActions(on_close: EventHandler<()>) -> Element {
    rsx! {
        footer { class: "flex justify-end gap-2",
            Button {
                r#type: "button",
                variant: ButtonVariant::Ghost,
                onclick: move |_| on_close.call(()),
                "取消"
            }
            Button { r#type: "submit", "保存" }
        }
    }
}
