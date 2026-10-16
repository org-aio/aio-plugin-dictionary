# AIO 字典插件

独立交付的 v2 整包：同一仓库提供 Dioxus 页面、原生 Rust Sidecar 和 PostgreSQL 迁移。安装后由运行时贡献“系统 / 字典管理”，升级、回滚、停用和卸载均无需重编 `aio-idea`。宿主不再链接字典客户端或服务端实现。

所有入口要求宿主角色权限 `dictionary:manage`；安装包本身不授予此权限。页面发现、挂载、资源和业务请求复用运行时的实时权限检查。进程只接受 `az-plugin-process` 验证后的调用身份，使用宿主发放的租户隔离数据库角色，不依赖产品身份实现。

## 阅读顺序

1. `aio-plugin.toml`：运行目标、前端目录、权限和数据库迁移声明。
2. `server/src/hosting.rs`：一个 Dill 容器装配现有 Controller/Service，监听监督器 Unix socket。
3. `client/src/main.rs`：共享样式和字典页面；请求通过宿主 `aioPlugin.json`。
4. [构建、测试和迁移](tests/README.md)：独立交付验收及旧宿主切换步骤。

`model` 保存共享 DTO；`server/src/generated/dictionary` 保存既有校验、事务与 CRUD；`client/src/dictionary` 保存类型选择、表格和按需打开的增删改对话框。没有第二套业务实现或请求桥。

## 构建与发布

使用仓库固定的 Rust nightly、Dioxus CLI 0.7.9、Node 22+、Zig 0.16.0、cargo-zigbuild 0.23.4。平台 `delivery/images/rust-process.Dockerfile` 提供组合工具环境，配置到构建服务的 `AIO_BUILD_IMAGE_RUST`。

```sh
sh scripts/build.sh --process
aio plugin validate .
aio plugin package . --git https://github.com/org-aio/aio-plugin-dictionary.git --version 0.1.0 -o dictionary.aio-plugin
```

脚本生成 `dist/dictionary-server`、`dist/dictionary-migrate` 和 `dist/frontend`。前端保留 DX 处理后的资源映射，仅将启动资源地址改为挂载路径下的相对地址；不能对 Cargo 原始 Wasm 重新执行 wasm-bindgen，也不能把共享 CSS 按文件名拼接。迁移工具供部署者使用，不作为插件运行入口。

发布者通过正式二进制发布接口上传包，经运行时校验后在插件市场按租户安装。`aio-delivery.toml` 声明相同构建命令；仓库 push、CI 通过、市场发布和租户激活是不同状态。运行环境必须允许清单固定的进程镜像，并提供进程监督器及受限数据库代理。不要把本包加入仅支持旧版发现的默认 Git 组合。

## 业务接口

以下路径位于 Sidecar 内部，经宿主受控桥访问；宿主根路径不注册字典 API：

- `GET /api/dictionaries`
- `POST /api/dictionaries/types`
- `PUT|DELETE /api/dictionaries/types/{type_id}`
- `POST /api/dictionaries/items`
- `PUT|DELETE /api/dictionaries/items/{item_id}`

租户取自可信调用上下文。类型编码在租户内唯一，每种类型最多一个默认项；删除类型级联删除其字典项。停用、重启和回滚保留数据；版本回滚不逆向执行数据库迁移。
