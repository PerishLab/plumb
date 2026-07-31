# 迁移到 Plumb v0.18.8

删除仓库现有 release 或 site wrapper 之前，先安装 stable Plumb。只有替代 CLI
已经可用，Doctor 才会接受 wrapperless 形状。

人工发布操作统一使用：

```sh
plumb release dispatch --channel beta --version vX.Y.Z-beta.N --ref <branch>
plumb stable prepare --version vX.Y.Z
plumb stable pick --version vX.Y.Z --commit <full-sha>
plumb stable freeze --version vX.Y.Z
plumb release dispatch --channel stable --version vX.Y.Z \
  --promotion-channel beta --promotion-version vX.Y.Z-beta.N
plumb stable packport --version vX.Y.Z
```

exact channel 保留分支自由度；只有 stable 使用 `release/vX.Y.Z`。packport 完成后
不要删除 release 分支；它会继续作为不可变的源与审计边界。

Site 仓库保留 dispatch-only deploy workflow 和 `apps/*/wrangler.jsonc`，在 workflow
中安装 stable Plumb，然后调用：

```sh
plumb site plan
plumb site inspect
plumb site deploy
```

inspect 或 deploy 需要 `PLUMB_SITE_ACCOUNT`、`PLUMB_SITE_DOMAIN` 和
`PLUMB_SITE_TOKEN`。已知观测点无法访问公网时可设 `PLUMB_SITE_BLIND=true`；
binding 为 unknown 时仍然不能放行。

env-only Runseal profile 只承载 `RUNSEAL_REPO_*` 路径，guard 直接调用 Plumb 与
Ectropy。release、ship、cold-start wrapper 只有在所属 CLI 或控制面可用后才移除。
Plumb 只消费已经 provision 的发布 authority，永远不负责 mint 或同步。
