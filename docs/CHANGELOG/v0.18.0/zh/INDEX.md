# Plumb v0.18.0

## 动态 stable 装配

Exact 发布保持来源自由：workflow dispatch ref 是唯一选择的来源，被调用 workflow
的直接事件 ref 与 SHA 会把所有共享 job 绑定到同一个不可变 commit。Exact channel
不读取 release branch 状态。

只有 stable 发布绑定到 `release/vX.Y.Z`。本地操作者负责准备发布线、通过线性
`cherry-pick -x` 加入选定变更，并在 dispatch 前冻结。Plumb 会在发布前校验 stable
ref、 version、commit 与 promotion proof。

激活后，packport 通过保留拓扑的 merge 收敛到 `main`。已发布 stable commit
必须成为 `origin/main` 的祖先；这笔债务只阻塞下一次 stable 激活，不阻塞普通
`main` 工作或 exact 发布。已结算的 release branch 可以删除。

## 发布身份

不可变 exact seal 与 stable pointer 继续承担发布权威。新发布不再创建 Git
tag，stable 产品 workflow 的 repository contents 权限也降为只读。历史 tag
保持不变。

## Operator 支持

Plumb 现在接纳提供 `prepare`、`pick`、`freeze` 与 `packport` 的 Sealkit `^0.3.1`
版本线。 迁移期间仍接纳原有 `^0.2.1` 版本线。
