# Plumb v0.18.25

living web policy 现在根据被检查仓库的实际源码推导词汇边界。

- web 席位中出现 Svelte 源码时，接纳 `.svelte`，并排除生成的
  `.svelte-kit` 输出。
- 纯 Svelte 席位不再继承 TSX 授权；两种源码同时存在时仍同时接纳。
- frozen release commit 现在会运行 exact 与 stable coordinate 所要求的
  同一条 push guard。

因此 Svelte 仓库既能保留自己的组件词汇，也继续以 release commit 本身
作为通过 guard 的发布边界。
