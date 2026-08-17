# Plumb v0.24.0

## 渲染出的 lane 现在对着要运行它的 forge 受检

v0.23.0 渲染了四条 workflow，而它们**从未被 Forgejo 解析过**。其中三处形状根本跑不
起来，而本地任何测试都说不出来——lane 的 body 只有 forge 会读。

exact 发布 lane 从 tag push 起跑，于是推一个 tag 就会自己发布一趟，并与 operator 派
发的那趟撞车。它还不转发 guard 契约，而空的契约列表不是拒绝、是**静默的豁免**：
`release evidence` 会回答「不需要证据」。两处都是对它所替换的 thin caller 的退化。

每条 lane 都用发布出去的 manager 安装工具，装在 home 目录下，然后从一个 PATH 里没有
那个位置的 shell 里调用它。guard 挂在它的最后一条 proof 上，而 ship lane 会挂在它的
第一条。

ship lane 在 `workflow_call` 下声明 secrets，而这个 forge 在那里只认 `inputs` 与
`outputs`。整个文件因此不可规划：每次 push、每个 pull request 都产生一趟零秒失败、
没有 job、没有日志的 run，它留下的状态挡住了落地。它替换掉的那条共享 workflow 同样
从未声明过 secrets——callee 读的是 caller 映射进来的东西。

渲染现在会拒绝带有上述任一形状的 lane，并点名是哪一条。检查发生在文件被写出之前，
所以一次让已知致命形状复活的模板改动，会停在作者手里，而不是停在别人的发布里。

## 本产品经由自己的 lane 发布

`plumb` 渲染并携带自己的四条 workflow，dispatch 派发的就是渲染出的那两条，本仓库不
再触达共享 workflow 仓库。尚未渲染的仓库不受影响：它的下一次发布会在到达 forge 之前
就拒绝，点名缺失的 lane 与写出它的命令，而其余未渲染的 lane 仍然只是一条 note。

模块附件带着有序 packages 回来了，worker 与它一同加入，于是站点是一个被声明的介质，
而不再是一条自己的 lane。ship 在 registry token 之外，也转发 worker 需要的凭据。
