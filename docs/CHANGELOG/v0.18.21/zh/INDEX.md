# Plumb v0.18.21

本版本将 Grok 收成受管的 agent skill 席位。

- `plumb skill` 按发现 `~/.claude` 与 `~/.codex` 的同一规则发现 `~/.grok`，
  并把受管 brief 装到 `~/.grok/skills/plumb`。
- 所有权、ledger、force、upgrade 与 uninstall 不变。路径未入账时，即使
  marker 匹配也拒绝覆盖。
- Concord 与 Ectropy 在下一次把带 `skill` feature 的 `plumb` 解析到本
  stable 之后继承该席位。

本版本不改变 document strategy、release locking，也不改变受管席位只收
canonical stable 的默认。
