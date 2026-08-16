# Plumb v0.22.0

## 声明面覆盖每一种投影介质

Plumb 此前只声明二进制、skill 与 Cargo 一族。镜像、chart 与 module 三个附件在上
一版被各自撤回，理由各不相同，如今全部失效。

module 撤回是因为 canonical stable 把凭据原样交给 npm，于是 npm 发出
`Authorization: Bearer Bearer <token>`，registry 拒绝。v0.21.0 已把凭据解析挪进
各 adaptor，而发布用的那条 lane 现在跑的正是那一版。

镜像与 chart 的原因根本不在 Plumb 里。它们的动词跑 `docker build` 与
`helm package`，而承载它们的 job 镜像两个客户端都没有。一个产品可以声明
`[release.oci]`、通过自己的 doctor，然后无处可造。Forge 镜像现已带上两个客户端，
共享 lane 也点明了它们说话的端点。

因此 `plumb.toml` 在二进制与 skill 之外，声明镜像、chart、module 与 Cargo 一族。
六种介质，一份声明，一道门。

## capsule 只覆盖 capsule 能覆盖的

`AGENTS.md` 仍写着「每个发布动词都拒绝封了别的版本的 capsule」。这句话自 v0.21.0
起不再为真——那一版起门问的是 `Spec::binary()` 而不是环境变量：一个不声明二进制
形状的发布既不编译 capsule，也没有 authority 去存它，索要 capsule 等于拒绝一种
Plumb 自己定义的形状。

文档现在说的是代码在做的事。capsule 存在的地方规则照旧成立，不可能存在的地方，
动词转而对声明的投影面负责。发布仍然是一次发布中唯一不可逆的那一点。
