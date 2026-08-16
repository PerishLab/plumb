# 迁移到 Plumb v0.21.0

## 旧版 plumb 读不了声明了 account 的 manifest

`[release.oci]` 与 `[release.chart]` 新增**必填**的 `account`。发布声明拒绝未知字段，
因此比这一版更旧的 plumb 无法读取带它的 manifest，而每个 `plumb release` 与
`plumb ship` 动词都会先读 manifest。

只有当构建该仓库的 plumb 至少是这一版时，才去声明它。已经声明了这两个附件之一的
仓库必须补上：

```toml
[release.oci]
registry = "git.perish.top"
image = "perishlab/example"
account = "PerishFire"
```

plumb 自己的 manifest 这一版两个附件都不声明，理由相同。

## `PLUMB_RELEASE_REGISTRY_ACCOUNT` 已移除

账号是公开名字，现在来自声明。导出过这个变量的 lane 可以停；从未导出过的无需改动。
`PLUMB_RELEASE_REGISTRY_TOKEN` 不变，仍然承载凭据。

## 模块 adaptor 需要 pnpm

`ship npm pack` 与 `ship npm publish` 调用的是 `pnpm` 而非 `npm`。发布模块的 lane
必须能用 pnpm；共享二进制 lane 在存在 lockfile 时已经会启用 corepack。

一个要发布构建产物的模块应当声明 `prepack`——两个包管理器都会执行它。其余无需改动。

## doctor 可能开始报一份它此前接受的声明

两条规则成为机制化，因此一个仓库可能在**没有任何改动**的情况下变成 out of true：

- `release.spec-declared` 拒绝一份严格 spec 读不了的 `[release]` 表。最常见的形状是
  附件表写在了某个后续顶层键**之前**：TOML 会把那个键改挂到附件上。
- `release.attachment-deliverable` 在「没有共享 lane 交付它」或「本仓库没有调用任何
  交付它的 lane」时拒绝一个已声明的附件。

doctor 报告里的 `publishes` 现在列的是已声明的附件。一个有 `packages/*` 却没有模块
附件的仓库，不再被报成 `npm`。

规则目录现有 114 条：机制化 74、观察 1、仅散文 39。断言过这些数字的读取方需要更新。

## 部分失败的发布现在可以续跑

重跑一次发布，不再在它已经到达的介质上拒绝，也不再静默替换。内容与 registry 持有物
不一致的 chart 或 image 会**拒绝并同时报出两侧**，而不是覆盖。

## `plumb land` 在同一条分支上可以用第二次

同一条主题分支的第二次落地此前会等一个永远不会跑的守卫——落地分支被保留，而它那个
已合并的 PR 仍然匹配。首次落地的行为不变。
