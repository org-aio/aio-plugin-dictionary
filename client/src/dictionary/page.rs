use super::{
    dialogs::{ItemEditorDialog, TypeEditorDialog},
    http,
};
use aio_plugin_dictionary_model::{DictionaryItem, DictionaryType};
use az_ui_components::{
    admin::{
        AsyncResult, CollectionTable, DeleteRecordsDialog, PageHeader, PageSurface, RequestState,
        SortValue, StatusMessage,
    },
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    collection_tree::{CollectionTree, CollectionTreeData, CollectionTreeItemContext},
    data_table::{DataTableAlign, DataTableCellContext, DataTableColumn},
    input::Input,
    select::{Select, SelectItem},
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{Check, Minus, Pencil, Plus, RefreshCw, Trash2};

#[allow(non_snake_case)]
pub(crate) fn DictionaryPage() -> Element {
    let mut revision = use_signal(|| 0_u64);
    let dictionaries = use_resource(move || {
        let _ = revision();
        http::load()
    });
    let mut selected_type_id = use_signal(String::new);
    let mut type_search = use_signal(String::new);
    let mut status_filter = use_signal(|| "all".to_owned());
    let mut type_editor = use_signal(|| None::<Option<DictionaryType>>);
    let mut type_delete = use_signal(|| None::<DictionaryType>);
    let mut item_editor = use_signal(|| None::<Option<DictionaryItem>>);
    let mut item_delete = use_signal(|| None::<Vec<DictionaryItem>>);
    let mut selection = use_signal(Vec::<DictionaryItem>::new);
    let mut feedback = use_signal(|| None::<String>);
    let view = match dictionaries.read().as_ref().cloned() {
        Some(Ok(view)) => view,
        Some(Err(error)) => {
            return rsx! { PageSurface { RequestState { error, on_retry: move |_| revision += 1 } } };
        }
        None => return rsx! { PageSurface { RequestState {} } },
    };
    let selected = view
        .types
        .iter()
        .find(|item| item.id == selected_type_id())
        .or_else(|| view.types.first())
        .cloned();
    let type_id = selected.as_ref().map(|item| item.id.clone());
    let rows = selected
        .as_ref()
        .map(|item| item.items.clone())
        .unwrap_or_default();
    let filter = type_search().to_lowercase();
    let types = view
        .types
        .iter()
        .filter(|item| {
            format!("{} {}", item.name, item.code)
                .to_lowercase()
                .contains(&filter)
        })
        .cloned()
        .collect::<Vec<_>>();
    rsx! {
        PageSurface {
            PageHeader { title: "字典管理", detail: format!("{} 个字典类型", view.types.len()),
                Button { size: ButtonSize::Icon, variant: ButtonVariant::Outline, title: "刷新字典", aria_label: "刷新字典", onclick: move |_| revision += 1, RefreshCw {} }
                Button { variant: ButtonVariant::Outline, onclick: move |_| type_editor.set(Some(None)), Plus {} "新建类型" }
            }
            if let Some(message) = feedback() { StatusMessage { message } }
            div { class: "admin-split",
                aside {
                    label { class: "admin-field", span { "搜索类型" } Input { aria_label: "搜索字典类型", r#type: "search", value: type_search(), oninput: move |event: FormEvent| type_search.set(event.value()) } }
                    CollectionTree {
                        aria_label: "字典类型".to_owned(), data: CollectionTreeData::Collection(types),
                        item_key: |item: DictionaryType| item.id, selected_key: type_id.clone(),
                        on_select: move |item: DictionaryType| { selected_type_id.set(item.id); selection.set(Vec::new()); },
                        render_item: |context: CollectionTreeItemContext<DictionaryType>| rsx! {
                            span { class: "admin-tree-row", span { "{context.item.name}" } Badge { variant: BadgeVariant::Outline, "{context.item.items.len()}" } }
                        }, empty_text: "没有匹配类型".to_owned(),
                    }
                }
                section { class: "admin-section",
                    if let Some(current) = selected {
                        header { class: "admin-toolbar",
                            div { h2 { "{current.name}" } p { class: "admin-meta", "{current.code} · {current.description}" } }
                            div { class: "admin-actions",
                                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, title: "编辑类型", aria_label: "编辑类型", onclick: { let value = current.clone(); move |_| type_editor.set(Some(Some(value.clone()))) }, Pencil {} }
                                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Icon, title: "删除类型", aria_label: "删除类型", onclick: { let value = current.clone(); move |_| type_delete.set(Some(value.clone())) }, Trash2 {} }
                                Button { onclick: move |_| item_editor.set(Some(None)), Plus {} "新建字典项" }
                            }
                        }
                    } else { h2 { "暂无字典类型" } }
                    CollectionTable {
                        key: "{type_id:?}-{status_filter}", label: "字典项",
                        rows: rows.into_iter().filter(|item| status_filter() == "all" || item.enabled == (status_filter() == "enabled")).collect::<Vec<_>>(),
                        columns: columns(), row_key: |item: DictionaryItem| item.id,
                        search_text: |item: DictionaryItem| format!("{} {}", item.label, item.value),
                        sort_value: |(item, key): (DictionaryItem, String)| match key.as_str() {
                            "order" => SortValue::Number(item.sort_order.into()),
                            "value" => SortValue::Text(item.value), _ => SortValue::Text(item.label),
                        }, sortable: vec!["label".into(), "value".into(), "order".into()],
                        selected_keys: selection().iter().map(|item| item.id.clone()).collect::<std::collections::BTreeSet<_>>(),
                        on_selection_change: move |items| selection.set(items),
                        tools: rsx! { div { class: "admin-actions",
                            label { class: "admin-filter", "状态" Select { aria_label: "字典项状态", value: status_filter(),
                                options: [("all", "全部状态"), ("enabled", "启用"), ("disabled", "停用")].into_iter().map(|(id, label)| SelectItem::new(id, label)).collect(),
                                on_value_change: move |value| { status_filter.set(value); selection.set(Vec::new()); },
                            } }
                            if !selection().is_empty() { Button { variant: ButtonVariant::Outline, onclick: move |_| item_delete.set(Some(selection())), Trash2 {} "删除选中 ({selection().len()})" } }
                        } },
                        render_cell: move |context: DataTableCellContext<DictionaryItem>| {
                            let item = context.row;
                            match context.column.key.as_str() {
                                "label" => rsx! { "{item.label}" }, "value" => rsx! { code { class: "admin-code", "{item.value}" } }, "order" => rsx! { "{item.sort_order}" },
                                "status" => rsx! { div { class: "admin-badges", span { class: "admin-status", "data-enabled": item.enabled.to_string(), if item.enabled { Check {} "启用" } else { Minus {} "停用" } } if item.is_default { Badge { variant: BadgeVariant::Outline, "默认" } } } },
                                "actions" => { let edit = item.clone(); rsx! { div { class: "admin-actions",
                                    Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "编辑 {item.label}", aria_label: "编辑 {item.label}", onclick: move |_| item_editor.set(Some(Some(edit.clone()))), Pencil {} }
                                    Button { size: ButtonSize::IconSm, variant: ButtonVariant::Ghost, title: "删除 {item.label}", aria_label: "删除 {item.label}", onclick: move |_| item_delete.set(Some(vec![item.clone()])), Trash2 {} }
                                } } }, _ => rsx! {},
                            }
                        },
                    }
                }
            }
        }
        if let Some(value) = type_editor() { TypeEditorDialog { value, on_close: move |_| type_editor.set(None), on_saved: move |_| { type_editor.set(None); feedback.set(Some("字典类型已保存".into())); revision += 1; } } }
        if let Some(value) = type_delete() {
            DeleteRecordsDialog { title: "删除字典类型", warning: format!("同时删除该类型下的 {} 个字典项，无法撤销。", value.items.len()), items: vec![value],
                item_label: |item: DictionaryType| item.name, delete: |item: DictionaryType| -> AsyncResult<()> { Box::pin(async move { http::delete_type(&item.id).await }) },
                on_close: move |_| type_delete.set(None), on_deleted: move |_| { selected_type_id.set(String::new()); selection.set(Vec::new()); feedback.set(Some("字典类型已删除".into())); revision += 1; },
            }
        }
        if let (Some(value), Some(type_id)) = (item_editor(), type_id) {
            ItemEditorDialog { type_id, value, on_close: move |_| item_editor.set(None), on_saved: move |_| { item_editor.set(None); feedback.set(Some("字典项已保存".into())); revision += 1; } }
        }
        if let Some(items) = item_delete() {
            DeleteRecordsDialog { title: "删除字典项", items, item_label: |item: DictionaryItem| format!("{} ({})", item.label, item.value),
                delete: |item: DictionaryItem| -> AsyncResult<()> { Box::pin(async move { http::delete_item(&item.id).await }) },
                on_close: move |_| item_delete.set(None), on_deleted: move |count| { selection.set(Vec::new()); feedback.set(Some(format!("已删除 {count} 个字典项"))); revision += 1; },
            }
        }
    }
}

fn columns() -> Vec<DataTableColumn> {
    vec![
        DataTableColumn::leaf("label", "显示标签").width(180),
        DataTableColumn::leaf("value", "字典值").width(160),
        DataTableColumn::leaf("order", "排序")
            .width(100)
            .align(DataTableAlign::End),
        DataTableColumn::leaf("status", "状态").width(150),
        DataTableColumn::leaf("actions", "操作").width(100),
    ]
}
