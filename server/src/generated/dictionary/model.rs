#[derive(Debug)]
pub(super) struct DictionaryTypeRow {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug)]
pub(super) struct DictionaryItemRow {
    pub id: String,
    pub type_id: String,
    pub value: String,
    pub label: String,
    pub sort_order: i32,
    pub is_default: bool,
    pub enabled: bool,
}
