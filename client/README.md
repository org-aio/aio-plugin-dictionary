# 字典客户端

`main.rs` 独立启动既有 Dioxus 字典页面，在运行时的 v2 iframe 中加载。共享 `UiStylesheets { relative_paths: true }` 负责样式顺序；`scripts/package-frontend.mjs` 保留 DX 已处理的 JS/Wasm 和资源，确保路径相对于当前挂载版本。

`dictionary/http.rs` 通过 Dioxus 消息 API 调用宿主注入的 `aioPlugin.json`，不拼接用户输入、不读取会话 Cookie、不创建第二套请求桥。页面复用共享 UI 的表格、集合树和对话框。页面描述来自同包服务端 `/aio/describe`，无需产品静态注册。
