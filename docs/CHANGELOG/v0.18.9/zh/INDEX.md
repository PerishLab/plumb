# Plumb v0.18.9

## 精确变更边界证明

`plumb precommit` 与对应 library API 现在可以证明两个精确 commit OID 之间的
已提交 delta 是否完全落在显式 repo-relative 写路径前缀内。证明要求当前 head
干净；rename 与 copy 的新旧路径都会进入集合；versioned JSON 会返回规范化的
changed 与 outside 集合。

evaluator 不绑定协调器：它不读取任务状态，不创建 Git hook，也不分配 claim。

## 过渡态域词典

Doctor 现在报告随 Plumb release 锁定的 `p64-v1` retired 域词典，并检查
`docs/CHANGELOG/**` 之外的 tracked path 与当前 worktree bytes。命中为 out of
true，证据不可读为 blind；JSON 报告会绑定 dictionary digest 与扫描 coverage。

本版本刻意携带空 retired 集合，只建立机制，不启动任何词汇迁移。
