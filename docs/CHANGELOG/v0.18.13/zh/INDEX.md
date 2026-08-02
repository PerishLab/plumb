# Plumb v0.18.13

## Forgejo 可复用工作流终态

`plumb release dispatch --watch` 过去信任父 run 聚合，并向一个 GitHub 形状的 jobs
路由索取失败详情。Forgejo 可能在所有底层任务均成功后仍把可复用工作流的父 run 报为
`blocked`，而当前 Forgejo API 也不包含该 jobs 路由。

现在，仅当终态需要消歧或诊断时，watcher 才读取 Forgejo 的分页 action-task 面。它以
仓库内唯一的 run number 关联任务：全部成功的可复用工作流判为成功，可选的 skipped
任务保持中性，并点名失败、取消或阻塞的任务。未知或畸形的状态证据会拒绝，不会被猜成
成功。

既有的原始 `Client::run` API 保持不变；新增的 `Client::outcome` 与类型化 `Outcome`
投影承载这项判断。
