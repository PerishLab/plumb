# 迁移到 Plumb v0.23.0

## 模块附件声明 packages

`[release.npm]` 原先取 `package`，现在取 `packages`（有序列表）。只发布一个模块
的仓库写一个单元素列表。

## dispatch 只取版本

`plumb ship binary dispatch` 不再接受 `--promotion-channel` 与
`--promotion-version`，传入任一个都会解析失败。stable dispatch 只报版本。

## 受治理的 workflow 需要渲染

运行 `plumb lane --write` 并落地。在此之前，`doctor` 会把每条受治理的 lane 记为
absent，这既不改变退出码也不阻塞发布。一旦渲染过，再手工编辑它，下一次 dispatch
就会拒绝：渲染过又被改的 lane 是对「实际会跑什么」的谎言，而从未渲染的只是尚未
采纳这套机制的仓库。

## 站点自己声明

`deploy.yml` 作为 workflow 变量传入的 account 与 domain 应写进
`[release.cfworker]`，token 仍留在 secret。仍在使用 `ship site deploy` 的仓库在
声明该附件之前不受影响。

## 封印新增字段但不改 schema

封印 schema 仍为 1。`inputs` 是可选的增量字段，旧 Plumb 读得了新封印，新 Plumb
也读得了旧封印。`ship binary smoke` 现在接受封印 URL 并按自身平台解析 manager；
传 manager URL 仍然可用。

## 未变的 crate 保留它已发布的版本

输入没有变化的工作区 crate 不会被重新发布，因此本次发布出去的 crate 可能要求它的
一个更早的精确版本。无需处理：那个版本注册表里已经有了。
