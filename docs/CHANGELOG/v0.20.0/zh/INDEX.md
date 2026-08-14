# Plumb v0.20.0

## 六个 adaptor 全部吸收

`ship` 此前只带 `binary`,另外五个介质只是被声明、被拒绝。现在六个都应答:
`binary`、`site`、`cargo`、`oci`、`chart`、`npm`。拒绝分支整个删掉了,因为
没有东西还需要被拒绝。

`cargo` 接走了 `plumb release registry` 手里那两个动词,于是持有真值周期的那个
对象不再投影。`site` 离开顶层,改为在 `ship` 之下应答。`oci`、`chart`、`npm`
是新的:每一个都把一次真实投影落到一个真实的 registry 上。

## 投影跑不到封印前面

每个 publish 动词现在都读已编译的 capsule,封的是别的版本就拒绝。于是发布成为
一次发布中唯一不可逆的那一点,而一个问题就能问尽全局:**没有已发布 seal 的版本,
在任何介质上都没有投影。**

`registry` 还站在 `ship` 之外时,这条性质并不成立——一条独立的 Cargo lane 可以
在完全没有 capsule 的情况下发出 crate,于是「这个版本有没有到达过某个介质」只能
靠枚举介质来回答,还得指望那份清单是全的。现在它由一次 seal 探测回答。

只做准备的动词不受这条约束,它们除工作区外什么都不改:`cargo rehearse`、
`oci build`、`chart package`、`npm pack`。

## stable 发布有了一个可移除的点

stable 此前只有线,没有点,于是一个已准备好的发布没有任何把手供撤回去抓——线是
永久审计边界,删掉它就毁掉了它存在的意义。`plumb stable freeze` 现在会在冻结的
线头打上 `vX.Y.Z`,并拒绝移动一个已经站定的点。

`plumb stable retract` 移除那个点。它先读发布权威,只有在读到干净的「不存在」时
才继续。读到 seal 就拒绝,因为这个版本投影过东西;读到其他任何答案也拒绝,因为
破坏性动作绝不建立在一个它无法信任的读数上。判据是状态码,不是措辞。

撤回移除的是一个名字。线原样站着,可以重新冻结。

## 三个载体不承接职能

`Containerfile`、`charts/plumb`、`packages/plumb` 的存在,是为了让 image、chart
与 module 三个 adaptor 投影到一个真实介质而不是一个被描述的介质。它们不承接任何
职能,没有任何 lane 消费它们,各自在内容挣到位置时才装内容。

## JSR 退役,站点转为 Svelte

已经没有任何东西发布到 JSR,因此也没有消费者能从那里解析出一个第一方包。该
registry、它的协议席位、以及读 `deno.lock` 的 radius 维度一并移除;radius 现在
只读 Cargo 的 lock。站点随这次退役转到 Svelte,web 形状也跟着移动:web 应用携带
`@perish/design`,视图与组件是 `.svelte` 文件。
