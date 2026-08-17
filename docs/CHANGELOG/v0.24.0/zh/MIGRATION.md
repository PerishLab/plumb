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
