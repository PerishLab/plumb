# Plumb v0.18.19

Plumb 现在统一管理一套闭合文档面。`agent`、`architecture`、`design` 与
`brief` 的目标文件及文本量级均由二进制固化；下游仓库只声明策略、客观
source seat 与人工确认的 seal。

Doctor 只读取一次 Git 索引和工作树字节，并将同一份快照同时交给词汇与
文档规则。它会报告所有位于已声明目标及 `docs/CHANGELOG/**` 之外的受跟踪
Markdown，并精确指出使目标失效的 source binding。

`plumb document` 只输出 source 与 target seal 提案，不改写 manifest。
target seal 同时绑定策略、brief 名称、有序 source 拓扑与目标字节；每条
source seal 独立保存，因此变更来源可以直接暴露。

exact release 编译会比较上一个公开 stable commit 与冻结 candidate。每种
语言的 `INDEX.md` 与 `MIGRATION.md` 共用
`clamp(4 * ceil(sqrt(diff_units)), 120, 800)` 上限；diff units 包含文本增删
和变更路径数。release seal 会保留 base、candidate、diff identity、量级与
双语文档 seal。

本版本是有界兼容桥。仅在不存在 `[[document]]` 时继续接受旧 `[skill]` 与
`[[lock]]`，两套模型不可共存。所有受管仓库迁移完成后，下一 enforcement
版本将删除旧 schema 与 `plumb lock`。
