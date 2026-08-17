# 迁移到 Plumb v0.24.0

## 下一次发布之前先渲染 lane

`plumb ship binary dispatch` 现在派发渲染出的 `exact.release.yml` 与
`stable.release.yml`。仍然携带通往共享 workflow 的 thin caller 的仓库，会在到达
forge 之前就拒绝，并点名要跑的命令：

```bash
plumb lane --write
```

落地它，然后再派发。这件事没有任何全域性，所以仓库在它的 operator 想发布时迁移，而
不是按别人定的时间表。渲染出的 lane 到位之后，删掉 `release-exact.yml`、
`release-stable.yml` 与可能存在的 `deploy.yml`；站点改为 `[release.cfworker]`。

## v0.23.0 渲染出的 lane 必须重渲一次

v0.23.0 写出的 lane 至少带有一处这个 forge 跑不起来的形状。任何已经用 v0.23.0 跑过
`plumb lane --write` 的仓库，都应当用本版本再跑一次并落地。doctor 会把差异报为 drift，
而发布会在「由更旧的 Plumb 渲染出的 lane」上拒绝。

## worker 介质需要转发它的凭据

`[release.cfworker]` 经由 ship lane 投影，而 ship 现在在 registry token 之外还接受一个
site token。把 `PLUMB_SITE_TOKEN` 配成仓库 secret；渲染出的两条发布 lane 会转发它。

## 渲染出的 lane 只能花掉 canonical stable 会的动词

渲染出的 lane 装的是 canonical stable Plumb，所以它花掉的每个动词都必须存在于**已经
发布出去**的那一版里。投影矩阵从本版起携带准备动词；渲染这些 lane 的仓库，是用**读得
懂它们的那个 Plumb**（即本版或更晚）来发布的。

渲染之前要知道两件事。**带 binary 的镜像目前还不能经 ship lane 投影**：投影 job 手里
没有归档，而包着归档的镜像要从归档里取 payload。以及本产品的 v0.24.0 是**最后一次**
经由共享 workflow 发布，因为它现在携带的 lane 要求一份只有本版才产出的计划。
