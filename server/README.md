# 字典服务端

`main.rs → hosting::run` 读取监督器下发的配置，用同一个 Dill 容器装配 `PostgresDictionaryService` 与 `DictionaryController`，通过 `Host::protect` 验证入口 token、租户、用户和交互上下文。宿主在分发前检查实时 `dictionary:manage` 权限，服务端不链接产品身份插件。

`/health` 和 `/aio/describe` 供运行时健康检查与目录发现；业务路由全部位于受保护区域。`server/migrations` 是表结构唯一来源，由运行时先行执行，进程只验证受限数据库角色、schema 和租户隔离。SIGTERM/SIGINT 排空请求后关闭本代连接池。

`dictionary-migrate` 是部署者离线工具，从旧宿主复制选定租户的数据。它查询正式安装绑定取得目标 schema，使用表锁、迁移锁及单个目标事务；相同数据可重试，不同数据冲突使整批回滚。旧表不删除。操作及验收见 [测试说明](../tests/README.md)。
