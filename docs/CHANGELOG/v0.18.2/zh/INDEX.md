# Plumb v0.18.2

## 显式目标观察

Plumb 现在可以把一项由调用方选择的 CLI 目标事实绑定到产品自有的
`plumb.target` context role。通过 Locus 0.1.1，显式有序 collector chain
可选择一个具名环境值、一个指定序号的参数，或一项具名进程事实。

该能力默认仍然关闭。除非调用方精确选择对应事实，Plumb 不会推断仓库身份、
扫描 argv 或环境，也不会暴露工作区路径。

## 冻结来源

start Atom 记录产生该事实的 role、binding、collector 与 selector。finish Atom
继承同一个 readonly context，不会重复采样。显式 collector chain 只有在事实
缺席时才会继续；非法或不可读事实会通过 Locus diagnostic hook 拒绝审计记录。
