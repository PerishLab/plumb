# 迁移到 Plumb v0.18.27

不再依赖 `~/.tea/tea.yml` 为 Plumb 提供凭据。显式提供 Forgejo authority 与一个
token 座位：

```bash
export FORGEJO_URL=https://git.perish.top
export FORGEJO_TOKEN_FILE=/path/to/forgejo-token
```

`FORGEJO_TOKEN` 继续供 CI 与有边界的测试使用。操作者优先使用 token file，文件中
只包含 token 值。

命令语法没有变化。`plumb land`、`plumb release dispatch`、`plumb stable`、
recovery 与 retire 保留原有编排和拒绝语义，同时使用 Runseal 的进程内 Forgejo 方言。
