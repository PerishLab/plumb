# 迁移到 Plumb v0.18.2

如果 CLI 审计保持关闭，或现有 trace/report 设置已经足够，则无需任何操作。

如需为每次 invocation 附加 opaque target label，请由调用方提供 label，并且只
选择这个具名环境值：

```sh
PLUMB_AUDIT_TARGET=repository-a \
PLUMB_LOCUS_TARGET_COLLECTORS=environment:PLUMB_AUDIT_TARGET \
PLUMB_LOCUS_REPORT_FILE=/tmp/plumb-audit.jsonl \
plumb doctor .
```

调用方需要审查每项被选事实的敏感性与稳定性。collector chain 只在事实缺席时
继续；非法的现存事实或 collector 错误会拒绝审计记录，不会静默切换到另一来源。
完整的有界语法见 `docs/audit.md`。
