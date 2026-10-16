# 字典整包验收与迁移

## 自动检查

CI 对服务端执行格式、Clippy、真实 PostgreSQL CRUD、独立 Unix socket 进程、身份拒绝、旧数据复制、重复迁移、冲突回滚和退出后连接回收；Web 作业构建实际 DX 产物并验证打包入口。

```sh
cargo fmt --all --check
cargo clippy -p aio-plugin-dictionary-server --all-targets -- -D warnings
cargo clippy -p aio-plugin-dictionary-client --target wasm32-unknown-unknown -- -D warnings
```

准备独立本机 PostgreSQL 库 `dictionary_test`，撤销 `public` schema 的公共权限，将 `AIO_TEST_DATABASE_URL` 指向它，然后运行：

```sh
cargo test -p aio-plugin-dictionary-server -- --include-ignored --test-threads=1
```

需要数据库的测试默认显式忽略，不把缺少数据库报告为通过。集成测试会创建临时角色、schema 和模拟旧表；完成后删除专用库或整个测试容器。

## 实际产品与 Sidecar

`runtime/delivery.py` 启动真实 `aio-idea` 与独立监督器，发布两版整包并验证：安装、旧数据复制、调用、前端版本切换、升级、回滚、跨重启停用、重新启用保留回滚版本及排除记录、恢复后数据保留、实时撤权、卸载、正常退出，以及宿主二进制摘要没有变化。它只接受本机 `dictionary_delivery_test` 空库，不能指向业务数据库。

先完成整包构建，准备未静态链接字典的产品二进制及 DX Web 产物。使用当前平台 `lib/plugin/bundle/examples/package.rs` 构建测试打包工具，避免旧 CLI 不识别新权限契约。配置以下环境变量后执行 `python3 tests/runtime/delivery.py`：

| 变量 | 用途 |
| --- | --- |
| `AIO_DICTIONARY_TEST_HOST` | 产品服务端二进制绝对路径 |
| `AIO_DICTIONARY_TEST_DATABASE_URL` | 本机专用 `dictionary_delivery_test` 连接 |
| `AIO_DICTIONARY_TEST_POSTGRES_CONTAINER` | 该专用 PostgreSQL 容器名称 |
| `AIO_DICTIONARY_TEST_WEB` | 产品 DX Web public 目录 |
| `AIO_DICTIONARY_TEST_PACKAGER` | 平台 package 示例二进制 |
| `AIO_DICTIONARY_TEST_MIGRATOR` | 本包 `dist/dictionary-migrate` |
| `AIO_DICTIONARY_TEST_BROWSER_CONTAINER` | 可选 Playwright 容器，仓库挂载于 `/workspace`，与验收宿主共享网络命名空间 |

浏览器容器运行 `npm ci`，使用与锁文件一致的 Playwright。`browser/dictionary.cjs` 通过真实产品 iframe 验证桌面和手机的类型/字典项 CRUD、默认项唯一性、筛选、搜索、排序、部分删除失败重试、级联删除确认、共享样式、页面溢出和控制台。截图及报告写入忽略目录 `test-results`。不提供浏览器容器时只完成协议及生命周期验收。

共享宿主频繁创建网卡时，Chrome 可能因网络切换中断回环请求。Linux 验收可让浏览器使用专用 PostgreSQL 容器的网络命名空间，并通过 `nsenter --target <PostgreSQL 容器 PID> --net` 启动本脚本；测试数据库连接改为该命名空间内的 `127.0.0.1:5432`。产品、浏览器和数据库仍通过真实 HTTP/PostgreSQL 通信，保留全部控制台错误断言。

## 从静态字典切换

1. 备份旧宿主数据库、组件存储数据库、部署配置及启动恢复记录，停止旧字典写入。先在隔离环境完成以下步骤，再按产品部署流程操作。
2. 使用支持 `required_permissions` 和原生进程 SDK 的运行时发布、安装本包，按每个需要保留字典数据的租户建立正式绑定；确认管理角色保留 `dictionary:manage`。如果旧配置明确停用了字典，保持该租户整包停用。
3. 为部署者工具设置 `AIO_LEGACY_DATABASE_URL`（旧宿主数据库）和 `AIO_COMPONENT_DATABASE_URL`（组件存储管理连接），执行 `dist/dictionary-migrate <tenant-id>`。工具不会推断 schema、删除旧行或覆盖不同的目标数据；冲突必须先处理，再重试。
4. 旧 `AIO_RUNTIME_COMPOSITION` 含 `dictionary` 字段时，在宿主停服期间用新二进制执行 `aio-idea runtime migrate`。它只移除此退役字段，保留其他启停配置，取得与宿主相同的独占文件锁；实际启动成功前不会替换最后成功记录。新部署输入不再接受 `dictionary`。
5. 启动新产品，确认旧根路径 `/api/dictionaries` 为 404；通过市场页面与正式整包入口核对各租户类型、项、默认值和权限。再完成后续写入切换。

包回滚只恢复实现与前端版本，不回滚业务数据。旧表保留用于核对，后续写入不会双写旧表；恢复旧宿主前须单独安排数据迁移，不能直接回退二进制而丢失新写入。以上生产切换需要部署方完成，本仓库的隔离测试不会修改生产数据。
