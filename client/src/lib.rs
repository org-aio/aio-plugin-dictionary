#![forbid(unsafe_code)]

mod dictionary;

use az_dioxus_admin_shell::{
    ApplicationMenuGroup, ApplicationPage, ApplicationPlugin, ApplicationScene,
};
use dill::CatalogBuilder;

#[derive(Debug)]
pub struct DictionaryPlugin;

impl ApplicationPlugin for DictionaryPlugin {
    fn pages(&self) -> Vec<ApplicationPage> {
        vec![ApplicationPage {
            id: "dictionary-management",
            label: "字典管理",
            icon: Some("book-open"),
            scene: ApplicationScene {
                id: "system",
                label: "系统",
            },
            menu_path: vec![ApplicationMenuGroup {
                id: "system-management".to_owned(),
                label: "系统管理".to_owned(),
                icon: Some("settings".to_owned()),
            }],
            required_permission: Some("dictionary:manage"),
            render: dictionary::DictionaryPage,
        }]
    }
}

pub fn register(builder: &mut CatalogBuilder) {
    builder
        .add_value(DictionaryPlugin)
        .bind::<dyn ApplicationPlugin, DictionaryPlugin>();
}

#[cfg(test)]
mod tests {
    use az_dioxus_admin_shell::collect_application_pages;

    use super::*;

    #[test]
    fn contributes_dictionary_under_system_management_tree() {
        let mut builder = dill::Catalog::builder();
        register(&mut builder);
        let pages = collect_application_pages(&builder.build()).expect("应能聚合字典页面");

        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].scene.id, "system");
        assert_eq!(pages[0].menu_path.len(), 1);
        assert_eq!(pages[0].menu_path[0].id, "system-management");
        assert_eq!(pages[0].required_permission, Some("dictionary:manage"));
    }
}
