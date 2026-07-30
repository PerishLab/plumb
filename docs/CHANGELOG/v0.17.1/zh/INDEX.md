# Plumb v0.17.1

Deno 会在 lockfile 中写入语义等价的规范化 requirement。尤其是 `^0.2.1`
这样的 pre-1.0 caret，会被记录为 `~0.2.1`。Plumb v0.17.0 只按声明原文找
锁键，因此会把一个有效、冻结在 Sealkit `0.2.1` 的 resolution 报成 blind。

Plumb 现在先找声明对应的精确锁键；若 Deno 做了规范化，则跟随 direct workspace
dependencies 中唯一的 Sealkit 条目。它仍然只判断本地字节、从不编辑 lock；当
direct dependency 缺失或存在歧义时也会拒绝，而不是在候选项之间猜测。
