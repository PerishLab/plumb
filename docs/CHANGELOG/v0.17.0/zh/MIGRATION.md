# 迁移到 v0.17.0

## 如果你的仓库发布 crate

声明发布面让 Plumb 看得见，并把 operator 入口命名为 `release`：

```toml
[release.cargo]
registry = "perish"
packages = ["your-macro", "your-lib"]
```

`ship` 是站点部署，`release` 是 registry 发布；两半都已机械化，所以一旦发布面
可见，用错词的 wrapper 会被报出来。

## 如果你的仓库声明了 release 表

无需处理。本工坊中所有声明 `[release]` 的仓库都写了 `binaries`，因此更精确的
判定不改变任何结论。此前只声明 release 表而没有 binaries 的仓库会被算作 binary
发布者，现在不会了——这正是要修的东西。

## 如果你在升级 Plumb 之前先改了发布 crate 的仓库

请在装上本版本之后再声明 cargo 面。旧版 Plumb 会把任何 release 键读成 binary
发布，并要求一个 crate 家族用不上的 `release-exact` 与 `release-stable` lane。
