# lls-addon-install

`lls-addon-install` 是一个 Rust 命令行工具，用于从官方 `LuaLS/LLS-Addons` 注册表安装 LuaLS addon 到本地目录，并自动更新 `.luarc.json`，让 LuaLS 能发现这些 addon。

## 警告

这个项目包含 AI 生成内容。

在正式使用前，你应当自行审查源代码、在自己的环境中验证行为，并确认生成或修改的 `.luarc.json` 符合你的项目预期，再决定是否将它用于生产环境或团队工作流。

## 功能说明

这个工具遵循 LuaLS 官方 addon 工作方式，而不是把 `LLS-Addons` 索引仓库本身复制到目标目录：

1. 从官方 `.gitmodules` 拉取 `LLS-Addons` 注册表元数据
2. 将 `openresty`、`love2d` 这类 addon 名称解析为真实 Git 仓库
3. 将每个 addon 克隆到 `--target/<addon-name>`
4. 删除克隆结果中的 `.git` 目录，使结果成为纯 addon 文件夹
5. 更新 `.luarc.json`，把 addon 根目录追加到 `workspace.userThirdParty`

这与 LuaLS 的推荐模型一致：多个 addon 位于同一个父目录下，而 LuaLS 通过 `workspace.userThirdParty` 引用这个父目录。

## 特性

- 按注册表名称安装一个或多个 addon
- 从官方注册表列出可用 addon
- 使用 `--force` 覆盖已存在的 addon 目录
- 默认自动更新 `.luarc.json`
- 支持带注释和尾随逗号的 JSONC `.luarc.json`
- 基于 `jsonc-parser` 的 CST 编辑，尽量保留现有注释和格式

## 依赖

- Rust
- `git`

## 构建

```bash
cargo build --release
```

## 用法

安装一个或多个 addon：

```bash
lls-addon-install --target ./LuaAddons openresty love2d
```

安装 addon 并更新指定 `.luarc.json`：

```bash
lls-addon-install --target ./LuaAddons --luarc /path/to/project/.luarc.json openresty
```

列出注册表中的 addon：

```bash
lls-addon-install --target ./LuaAddons --list
```

覆盖已安装内容：

```bash
lls-addon-install --target ./LuaAddons --force openresty
```

只安装，不修改 `.luarc.json`：

```bash
lls-addon-install --target ./LuaAddons --no-luarc openresty
```

查看帮助：

```bash
lls-addon-install --help
```

## `.luarc.json` 行为

如果 `.luarc.json` 不存在，工具会创建一个类似下面的文件：

```json
{
  "workspace.userThirdParty": [
    "/path/to/project/LuaAddons"
  ]
}
```

行为细节：

- 如果已经存在 `workspace.userThirdParty`，工具会追加路径，而不是覆盖。
- 如果文件使用旧键名 `Lua.workspace.userThirdParty`，工具会保留并更新这个键。
- `.luarc.json` 按 JSONC 解析，因此支持注释和尾随逗号。
- 文件通过 `jsonc-parser` 的 CST API 进行编辑，因此会尽量保留现有注释和格式。

## 参考资料

- LuaLS addon 文档: https://luals.github.io/wiki/addons/
- LuaLS 配置文档: https://luals.github.io/wiki/configuration/
- 官方 addon 注册表: https://github.com/LuaLS/LLS-Addons
