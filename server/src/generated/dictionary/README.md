# 字典功能

实现租户级字典类型与字典项 CRUD。Controller 从 `az-plugin-process::Caller` 读取可信租户并转发 HTTP；Service 实现负责校验、事务和 PostgreSQL 持久化。权限由宿主实时检查，业务模块不依赖产品身份实现；表结构只保存在 `server/migrations`。
