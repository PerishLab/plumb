# Plumb v0.18.11

## 主题分支落地

`plumb land` 将当前干净的主题分支落到基线分支。源分支不经变基直接推送，在
`land/<branch>` 派生一枚相对基线的单提交投影，待其 guard 成功后基线快进到该投影。
随后持有基线的那个独立工作树被同步到落地状态。

合并请求锁定投影的确切 head，并且从不请求 Forgejo 删除分支。保留写在机制里，调用方
无法通过任何开关请求相反行为。`--dry-run` 只打印计划动作，不触碰 Git 与远端；
`--no-watch` 在 PR 建立后即停止。

每一种拒绝都带类别：游离 HEAD、基线分支自身、发布线、脏工作树、缺失上游、无实质
改动、合并冲突、投影守卫期间源或基线发生移动，以及 guard 失败或持续挂起。

## Forgejo 与 Git 底座

Forgejo 客户端、Git 远端辅助、令牌席位、分支保护与 Actions 工作流面已从 stable
operator 命令下沉到 `plumb::forge` 库。`plumb::land` 建立其上，协调面无需子进程即可
调用。stable operator 的命令面保持不变。

`Client::settle` 接受显式合并策略：packport 路径保留合并提交，落地使用仅快进。
两者都不得删除分支。
