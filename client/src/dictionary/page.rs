use aio_plugin_dictionary_model::{DictionaryItem, DictionaryType};
use az_ui_components::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    collection_tree::{CollectionTree, CollectionTreeData, CollectionTreeItemContext},
    data_table::{DataTable, DataTableCellContext, DataTableColumn},
};
use dioxus::prelude::*;

use super::{
    dialogs::{DeleteItemDialog, DeleteTypeDialog, ItemEditorDialog, TypeEditorDialog},
    http,
};

#[allow(non_snake_case)]
pub(crate) fn DictionaryPage() -> Element {
    let mut revision = use_signal(|| 0_u64);
    let dictionaries = use_resource(move || {
        let _ = revision();
        http::load()
    });
    let mut selected_type_id = use_signal(String::new);
    let mut type_editor = use_signal(|| None::<Option<DictionaryType>>);
    let mut type_delete = use_signal(|| None::<DictionaryType>);
    let mut item_editor = use_signal(|| None::<Option<DictionaryItem>>);
    let mut item_delete = use_signal(|| None::<DictionaryItem>);

    let Some(result) = dictionaries.read().as_ref().cloned() else {
        return rsx! { p { "正在读取字典" } };
    };
    let view = match result {
        Ok(value) => value,
        Err(error) => return rsx! { p { role: "alert", "加载字典失败：{error}" } },
    };
    let selected = view
        .types
        .iter()
        .find(|item| item.id == selected_type_id())
        .or_else(|| view.types.first())
        .cloned();
    let selected_key = selected.as_ref().map(|item| item.id.clone());
    let selected_for_edit = selected.clone();
    let selected_for_delete = selected.clone();
    let selected_for_item = selected.as_ref().map(|item| item.id.clone());
    let rows = selected
        .as_ref()
        .map(|item| item.items.clone())
        .unwrap_or_default();
    let mut item_editor_signal = item_editor;
    let mut item_delete_signal = item_delete;
    let mut refresh = move || revision.set(revision().wrapping_add(1));

    rsx! {
        section { class: "grid gap-4",
            header { class: "flex flex-wrap items-center justify-between gap-3",
                div {
                    h2 { "字典管理" }
                    p { "维护当前租户可复用的字典类型和字典项。" }
                }
                div { class: "flex flex-wrap gap-2",
                    Button {
                        r#type: "button",
                        variant: ButtonVariant::Outline,
                        onclick: move |_| type_editor.set(Some(None)),
                        "新建类型"
                    }
                    Button {
                        r#type: "button",
                        disabled: selected_for_item.is_none(),
                        onclick: move |_| item_editor.set(Some(None)),
                        "新建字典项"
                    }
                }
            }
            div { class: "grid gap-4 md:grid-cols-3",
                aside { class: "grid content-start gap-3",
                    div { class: "flex items-center justify-between gap-2",
                        h3 { "字典类型" }
                        if let Some(value) = selected_for_edit {
                            div { class: "flex gap-1",
                                Button {
                                    r#type: "button",
                                    size: ButtonSize::Sm,
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| type_editor.set(Some(Some(value.clone()))),
                                    "编辑"
                                }
                                if let Some(delete_value) = selected_for_delete {
                                    Button {
                                        r#type: "button",
                                        size: ButtonSize::Sm,
                                        variant: ButtonVariant::Ghost,
                                        onclick: move |_| type_delete.set(Some(delete_value.clone())),
                                        "删除"
                                    }
                                }
                            }
                        }
                    }
                    CollectionTree::<DictionaryType> {
                        aria_label: "字典类型".to_owned(),
                        data: CollectionTreeData::Collection(view.types.clone()),
                        item_key: |item: DictionaryType| item.id,
                        selected_key,
                        on_select: move |item: DictionaryType| selected_type_id.set(item.id),
                        render_item: |context: CollectionTreeItemContext<DictionaryType>| rsx! {
                            span { class: "flex w-full items-center justify-between gap-2",
                                span { "{context.item.name}" }
                                Badge { variant: BadgeVariant::Outline, "{context.item.items.len()}" }
                            }
                        },
                        empty_text: "暂无字典类型".to_owned(),
                    }
                }
                main { class: "grid content-start gap-3 md:col-span-2",
                    if let Some(current_type) = selected {
                        div {
                            h3 { "{current_type.name}" }
                            p { "{current_type.code} · {current_type.description}" }
                        }
                    } else {
                        p { "创建字典类型后即可维护字典项。" }
                    }
                    DataTable::<DictionaryItem> {
                        aria_label: "字典项".to_owned(),
                        rows,
                        columns: columns(),
                        row_key: |item: DictionaryItem| item.id,
                        empty_text: "当前类型暂无字典项".to_owned(),
                        render_cell: move |context: DataTableCellContext<DictionaryItem>| {
                            let item = context.row;
                            match context.column.key.as_str() {
                                "label" => rsx! { span { "{item.label}" } },
                                "value" => rsx! { code { "{item.value}" } },
                                "order" => rsx! { span { "{item.sort_order}" } },
                                "status" => rsx! {
                                    div { class: "flex flex-wrap gap-1",
                                        Badge {
                                            variant: if item.enabled { BadgeVariant::Secondary } else { BadgeVariant::Outline },
                                            if item.enabled { "启用" } else { "停用" }
                                        }
                                        if item.is_default {
                                            Badge { variant: BadgeVariant::Outline, "默认" }
                                        }
                                    }
                                },
                                "actions" => {
                                    let edit_value = item.clone();
                                    let delete_value = item;
                                    rsx! {
                                        div { class: "flex gap-1",
                                            Button {
                                                r#type: "button",
                                                size: ButtonSize::Sm,
                                                variant: ButtonVariant::Ghost,
                                                onclick: move |_| item_editor_signal.set(Some(Some(edit_value.clone()))),
                                                "编辑"
                                            }
                                            Button {
                                                r#type: "button",
                                                size: ButtonSize::Sm,
                                                variant: ButtonVariant::Ghost,
                                                onclick: move |_| item_delete_signal.set(Some(delete_value.clone())),
                                                "删除"
                                            }
                                        }
                                    }
                                }
                                _ => rsx! {},
                            }
                        },
                    }
                }
            }
        }
        if let Some(value) = type_editor() {
            TypeEditorDialog {
                value,
                on_close: move |_| type_editor.set(None),
                on_saved: move |_| {
                    type_editor.set(None);
                    refresh();
                },
            }
        }
        if let Some(value) = type_delete() {
            DeleteTypeDialog {
                value,
                on_close: move |_| type_delete.set(None),
                on_deleted: move |_| {
                    type_delete.set(None);
                    selected_type_id.set(String::new());
                    refresh();
                },
            }
        }
        if let (Some(value), Some(type_id)) = (item_editor(), selected_for_item) {
            ItemEditorDialog {
                type_id,
                value,
                on_close: move |_| item_editor.set(None),
                on_saved: move |_| {
                    item_editor.set(None);
                    refresh();
                },
            }
        }
        if let Some(value) = item_delete() {
            DeleteItemDialog {
                value,
                on_close: move |_| item_delete.set(None),
                on_deleted: move |_| {
                    item_delete.set(None);
                    refresh();
                },
            }
        }
    }
}

fn columns() -> Vec<DataTableColumn> {
    vec![
        DataTableColumn::leaf("label", "显示标签").width(180),
        DataTableColumn::leaf("value", "字典值").width(180),
        DataTableColumn::leaf("order", "排序").width(80),
        DataTableColumn::leaf("status", "状态").width(140),
        DataTableColumn::leaf("actions", "操作").width(160),
    ]
}
