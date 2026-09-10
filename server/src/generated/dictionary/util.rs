use anyhow::{Result, ensure};

pub(super) fn validate_type(code: &str, name: &str, description: &str) -> Result<()> {
    let code = code.trim();
    let name = name.trim();
    ensure!(
        (2..=64).contains(&code.len()),
        "类型编码长度必须为 2 到 64 个字符"
    );
    ensure!(
        code.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        }),
        "类型编码只能包含字母、数字、连字符、下划线和点"
    );
    ensure!(
        !name.is_empty() && name.chars().count() <= 80,
        "类型名称长度必须为 1 到 80 个字符"
    );
    ensure!(
        description.chars().count() <= 500,
        "类型说明不能超过 500 个字符"
    );
    Ok(())
}

pub(super) fn validate_item(value: &str, label: &str, sort_order: i32) -> Result<()> {
    let value = value.trim();
    let label = label.trim();
    ensure!(
        !value.is_empty() && value.chars().count() <= 255,
        "字典值长度必须为 1 到 255 个字符"
    );
    ensure!(
        !label.is_empty() && label.chars().count() <= 120,
        "字典标签长度必须为 1 到 120 个字符"
    );
    ensure!(
        (-1_000_000..=1_000_000).contains(&sort_order),
        "排序值超出允许范围"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_portable_type_codes() {
        assert!(validate_type("order.status", "订单状态", "").is_ok());
        assert!(validate_type("订单", "订单状态", "").is_err());
        assert!(validate_type("x", "订单状态", "").is_err());
    }

    #[test]
    fn validates_item_bounds() {
        assert!(validate_item("paid", "已支付", 10).is_ok());
        assert!(validate_item("", "已支付", 10).is_err());
        assert!(validate_item("paid", "已支付", i32::MAX).is_err());
    }
}
