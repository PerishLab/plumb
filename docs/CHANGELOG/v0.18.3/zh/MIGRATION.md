# 迁移到 Plumb v0.18.3

没有直接 `@perish` JSR 或 `perish` Cargo 依赖的仓库无需迁移。对于受管边，
请在 registry 可访问时运行 doctor，并在仓库下次被触及时移动每项过期的直接
依赖。

对于 Deno，请移除第一方 import 中的已发布版本：

```json
{
  "imports": {
    "@perish/sealkit": "jsr:@perish/sealkit"
  }
}
```

随后使用 Deno 刷新 frozen lock。对于 Cargo，保留预期的原生 SemVer
requirement；必要时显式放宽或推进它，并使用 Cargo 将精确 lock resolution
更新到 stable latest。不要手工编辑任何 lock。

当 registry、manifest 或 lock 无法证明当前版本时，Doctor 会有意拒绝。应恢复
证据，而不是引入缓存版本表、兼容线或临时例外。传递依赖过期应回到声明直接边的
上游仓库处理。
