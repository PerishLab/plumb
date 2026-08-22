# 迁移到 Plumb v0.30.0

## 重新渲染 Ectropy policy

依次运行 `plumb depot sync`、升级 Plumb、`plumb policy --write` 与
`ectropy .`。默认 path 上限从 4 变为 3。不要把全局值改回 4：应当压平没有
挣来的目录层，或增加一条窄 boundary，并在 note 中点名负责退役它的任务。

## 在本地验证发布声明

打开 release line 前运行 `plumb doctor . --json`。Doctor 现在会拒绝发布命令
无法解析的 release 表，也会点名没有被任何已调用 lane 交付的附件。准备写入
`[release]` 的 TOML 键必须位于附件表之前；最终权威是解析器，而不是书写约定。

## Stable line 门会读取网络

`plumb release prepare` 与 `plumb release freeze` 在检查 stable 祖先关系前会
fetch origin，`--dry-run` 也一样。dry-run 仍不会远端写入，但现在要求远端可读。

如果当前 main 不包含上一条 stable 点，先运行
`plumb release rejoin --version <version>`，检查其拓扑，再打开或冻结下一条线。

## 命令与库无需迁移

17 个顶层命令契约、121 个规则标识、公开库 API 与渲染 policy 形状构成 v0.30
兼容边界。除接受更紧的 path 预算外，本版本不要求下游修改源码。
