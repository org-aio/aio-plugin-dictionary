# 字典客户端

向 AIO 的系统场景贡献字典管理树菜单页面。页面使用共享 UI 组件展示字典类型和字典项，所有新增、编辑与删除操作通过按需挂载的 Dialog 完成。

Web 请求使用宿主同源地址；Desktop 请求使用 `AIO_API_BASE_URL`，并可通过 `AIO_SESSION_COOKIE` 注入已建立的宿主会话。
