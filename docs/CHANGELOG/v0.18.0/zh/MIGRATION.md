# 迁移到 Plumb v0.18.0

采用新发布 workflow 前先安装 Plumb v0.18.0，并把 release wrapper 移到 Sealkit
`^0.3.1`。版本刚发布时，用零依赖年龄下限刷新冻结 Deno lockfile：

```sh
deno install --config .runseal/deno.json --lock .runseal/deno.lock \
  --minimum-dependency-age=0
```

产品 exact 与 stable workflow 不再声明第二个 `ref` 输入。应在所需来源分支上
dispatch workflow，只传 exact version 或 promotion inputs。Caller 不转发来源值；
被调用的共享 workflow 直接读取其事件 ref 与 SHA。

在本地准备 stable 发布线：

```sh
runseal :release prepare --version vX.Y.Z
runseal :release pick --version vX.Y.Z --commit <sha>
runseal :release freeze --version vX.Y.Z
runseal :release --channel stable --version vX.Y.Z \
  --promotion-version vX.Y.Z-beta.N --watch
runseal :release packport --version vX.Y.Z
```

Stable dispatch 固定推导 `release/vX.Y.Z`，应移除 stable `--ref`。Packport
默认删除已结算 分支；只有在仍需用分支保留源码历史时才传 `--keep-branch`。

不要创建发布 tag。既有 tag 仍保留为历史记录，但不再参与发布共识。
