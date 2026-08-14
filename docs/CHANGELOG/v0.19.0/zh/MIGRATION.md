# 迁移到 Plumb v0.19.0

## 不要再向发布 lane 传 channel 与 version

caller 只转发晋升选择与 guard 证据：

```yaml
jobs:
  release:
    uses: PerishLab/actions/.forgejo/workflows/release-binary.yml@main
    with:
      guard_contexts: '["guard / guard (push)"]'
```

两个输入仍保留声明且不被读取，因此未改的 caller 在迁移期间继续工作。
等到没有 caller 再传它们时才会删除声明。

派发到承载身份的那个 ref：`release-exact.yml` 派发到 exact tag，
`release-stable.yml` 派发到冻结的发布线。派发前先推 tag——源绑定只接受
`refs/tags/<exact-version>`，不接受其他任何形式。

```bash
plumb ship binary dispatch --version v0.19.0-beta.1
```

## 补 v0.18.28 遗漏的迁移说明

那一版搬走了九个动词、并改变了 exact 的源绑定，却没有留下迁移说明，
而 stable 发布事后不可改写。该说明只能落在这里。

```text
plumb release build     -> plumb ship binary build
plumb release assemble  -> plumb ship binary assemble
plumb release matrix    -> plumb ship binary matrix
plumb release managers  -> plumb ship binary managers
plumb release publish   -> plumb ship binary publish
plumb release smoke     -> plumb ship binary smoke
plumb release verify    -> plumb ship binary verify
plumb release dispatch  -> plumb ship binary dispatch
plumb release recovery  -> plumb ship binary recovery
```

没有别名。旧调用在参数解析阶段失败，发生在任何网络写入之前，因此过期的调用方
只会拒绝，不会错误发布。`release` 保留 `source`、`compile`、`packport`、
`authority`、`promote`，以及尚未定下的 `activate`、`inspect` 与 `registry`。

exact 发布不再能绑定任意分支：它绑定命名其版本的那个 tag，
而 stable 发布绑定它的发布线。
