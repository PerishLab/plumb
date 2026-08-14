# 迁移到 Plumb v0.20.0

## 两族动词搬了座位

```text
plumb release registry publish   -> plumb ship cargo publish
plumb release registry rehearse  -> plumb ship cargo rehearse
plumb site plan                  -> plumb ship site plan
plumb site inspect               -> plumb ship site inspect
plumb site deploy                -> plumb ship site deploy
```

没有别名。旧调用在参数解析阶段失败,发生在任何网络写入之前,因此过期的调用方
只会拒绝,不会错误发布。`release` 保留 `source`、`compile`、`packport`、
`authority`、`channel`、`promote`,以及尚未定下的 `activate` 与 `inspect`。

## lane 该问二进制,不是问版本

lane 装的是 canonical stable Plumb,跑的是那一版手里有的动词。所以一条把某种
拼法写死的 lane,会在 stable 跨过这一版的那一刻挂掉。改成问二进制:

```sh
if plumb ship cargo publish --help >/dev/null 2>&1; then
  plumb ship cargo publish
else
  plumb release registry publish
fi
```

探到动词那一层,不要只探 adaptor。在 adaptor 已声明但仍拒绝的版本上,
`plumb ship cargo --help` 会成功,那样探测会选中一个根本跑不了的动词。

## 旧版 Plumb 读不了新 manifest

`plumb.toml` 新增可选的 `[release.oci]`、`[release.chart]`、`[release.npm]`。
发布声明拒绝未知字段,因此比这一版更旧的 Plumb 无法读取带有它们的 manifest,而
每个 `plumb release` 与 `plumb ship` 动词都会先读 manifest。

只有当构建该仓库的 Plumb 至少是这一版时,才去声明这些附件。不声明的仓库不受影响。

## JSR 已移除

第一方 JSR 解析随 registry、协议席位,以及读 `deno.lock` 与 `.runseal/deno.lock`
的 radius 维度一并移除。仍从 JSR 解析第一方包的产品在这里没有前路;radius 现在
只读 Cargo 的 lock,`plumb radius` 少报一个维度。

## stable 线会打点

`plumb stable freeze` 现在会在冻结的线头创建并推送 `vX.Y.Z`。点已经站在那个
commit 上时它幂等,站在别处时它拒绝。

```bash
plumb stable retract --version vX.Y.Z
```

撤回在什么都还没发布时移除那个点,一旦发布权威给出该版本的 seal 就拒绝。
它从不触碰线。
