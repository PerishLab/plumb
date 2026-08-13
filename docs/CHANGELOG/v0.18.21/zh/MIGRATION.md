# 迁移

已有的 Claude 与 Codex 席位无需操作。

若本机已有 `~/.grok`，下一次 `plumb skill install` 会尝试写入
`~/.grok/skills/plumb`。该路径已存在且不在受管账本里时会被拒绝。先移走
该目录再安装。`--force` 不能认领它。

没有 Grok 主目录的安装不受影响。
