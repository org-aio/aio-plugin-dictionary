# AIO 字典插件 / AIO Dictionary Plugin

面向 AIO 系统场景的租户级字典管理插件。一个 Git 仓库内聚 Dioxus 客户端、PostgreSQL 服务端和共享请求模型，通过 Dill 按具体 Rust 类型装配。

A tenant-level dictionary management plugin for AIO system scenarios. One Git repository hosts the Dioxus client, PostgreSQL server and shared request models, wired by Dill through concrete Rust types.

导航位置为 `系统 / 系统管理 / 字典管理`，访问与写操作均要求 `dictionary:manage` 权限。

Navigation: `系统 / 系统管理 / 字典管理` (System / System Admin / Dictionary Management). Both read and write operations require the `dictionary:manage` permission.

## 目录 / Layout

- `client`：字典类型集合、字典项表格及增删改 Dialog。
- `model`：前后端共享且与框架无关的 DTO。
- `server`：租户隔离的 PostgreSQL CRUD 与鉴权路由。

- `client`: dictionary type collections, dictionary item tables and add/edit/delete dialogs.
- `model`: framework-agnostic DTOs shared between the frontend and backend.
- `server`: tenant-isolated PostgreSQL CRUD and authenticated routes.

## 接口 / API

- `GET /api/dictionaries`
- `POST /api/dictionaries/types`
- `PUT /api/dictionaries/types/{type_id}`
- `DELETE /api/dictionaries/types/{type_id}`
- `POST /api/dictionaries/items`
- `PUT /api/dictionaries/items/{item_id}`
- `DELETE /api/dictionaries/items/{item_id}`
- `GET /api/plugins/dictionary/health`

宿主从会话注入当前租户，客户端不能提交或覆盖 `tenant_id`。每个租户内类型编码唯一，每种类型最多一个默认字典项，删除类型会级联删除其字典项。

The host injects the current tenant from the session; the client cannot submit or override `tenant_id`. Type codes are unique within each tenant, each type has at most one default item, and deleting a type cascades to its dictionary items.
