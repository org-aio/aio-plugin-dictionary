# 字典服务端

通过 Dill 装配 Controller 与 Service，以 PostgreSQL 保存租户级字典类型和字典项，并从身份插件会话中取得可信租户上下文。

可独立注册 `DictionaryController` 与 `PostgresDictionaryService`，把实例的创建、`initialize()` 和 `shutdown()` 交给宿主生命周期；控制器仍从同一个 Dill 容器注入 `DictionaryService` 与身份服务。关闭只释放本代连接池，重新启用创建新实例，字典数据保留。
