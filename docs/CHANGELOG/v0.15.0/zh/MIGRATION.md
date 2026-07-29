# 迁移到 v0.15.0

## 对安装 plumb 的人

无需动作。下一次 `manage.sh update` 会清掉它找到的旧版本目录，并打印删了哪些。

**你会失去的是离线回滚**。此前网络不通时，还能手工把符号链接指回磁盘上的旧目录；
现在回滚是 `manage.sh install --version <旧版本>`，需要能连上发布主机。已发布的
制品不可变且永久可取，所以代价是一次下载而不是一个版本——但在连不上
`releases.plumb.perish.uk` 的工作站上，这是实打实的损失，属于明知的取舍。

CI 不受影响：那些 runner 每次都从干净容器起，本来就没有可失去的本地副本。

## 对要发 stable 的仓库

`release-stable` 放行之前，现在有两件事是必需的。

**一、写变更日志。** 为即将发布的版本建立：

```
docs/CHANGELOG/v<version>/
├── en/
│   ├── INDEX.md
│   └── MIGRATION.md
└── zh/
    ├── INDEX.md
    └── MIGRATION.md
```

四份文件都必须存在且非空。`en` 与 `zh` 是最低要求，可以更多。

多数版本并没有任何人需要动手的变更。**仍然要写 MIGRATION.md**，把这件事明说：
「本版无需迁移」是有人读完 diff 之后得出的结论，它和一份没人写过的空文件不是
同一种东西。

**二、把门控加进 lane。** 在 `release-stable.yml` 的第一个不可逆步骤之前：

```yaml
      - name: Changelog
        env:
          RELEASE_VERSION: ${{ needs.metadata.outputs.release_version }}
        run: plumb changelog . --version "$RELEASE_VERSION"
```

plumb 自己用的是 `cargo run --quiet -p plumb-cli -- changelog`——一个工具无法
用尚不存在的自己来给自己的发布把关。

在你主动加之前没有任何东西逼你：`plumb doctor` 两种情况下都是绿的，今天也只有
本仓库带着这道门控。另外六个带 `manage.sh` 的仓库原封未动，何时采纳由它们自己决定。

## 不追溯

v0.15.0 之前的版本没有变更日志，也不会补写。它们的 diff 都在历史里；事后重建的
变更日志是把猜测当作记录呈现，比留白更坏。
