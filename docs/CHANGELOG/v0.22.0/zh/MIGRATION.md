# 迁移到 Plumb v0.22.0

## 声明镜像或 chart 附件，需要 job 镜像带着对应客户端

`ship oci build` 跑 `docker build`，`ship chart package` 跑 `helm package`。
声明 `[release.oci]` 或 `[release.chart]` 仅凭声明本身就能过 doctor，因此一个
lane 镜像不带这两个客户端的仓库，会声明出一种自己够不到的介质，并在动词处失败。

共享的 `release-binary` lane 现在跑的 Forge 镜像带着两个客户端，并为它们点明了
`DOCKER_HOST`。调用那条 lane 的仓库无需再做什么。自建 lane 的仓库必须先备好客户端
与端点，再去声明这两个附件之一。

## 两者都不声明的仓库，什么都不必做

镜像、chart 与 module 三个动词回到共享 lane，对它服务的每一个产品都生效。一个不
声明该附件的产品不打开端点、不做认证、不付任何代价：每个动词都回答自己没有，然后
成功。
