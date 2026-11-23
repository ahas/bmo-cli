[English](README.md) | [한국어](README.ko.md) | [日本語](README.ja.md) | **简体中文**

# bmo-cli

平台的轻量入口 `bmo`。它不提供 API,也不运行任何服务。每个单元都附带自己的命令,名为 `bmo-<unit>`;`bmo` 会像 git
查找 `git-<command>` 那样,找到提供某条命令的单元命令,并把命令交给它。之后新增的单元无需重新构建 `bmo`。

## 目录结构

| 目录 | 内容 |
|---|---|
| `src/` | 读取输入词语、查找单元命令及其名词、解析命令并转交的库,以及二进制文件 `bmo` |
| `tests/` | 契约测试,每个测试都在独立目录中让构建好的 `bmo` 对替身单元命令运行 |
| `docs/` | 关于 `bmo` 行为的权威文档,以及个人数据规则 |

根目录下有 Cargo 清单、版本文件、`testbed.toml`(bmo-testbed 的声明,说明本仓库如何测试、检查、构建和部署)、`install.toml`(说明在新机器上安装时需要什么,由测试台的 install 动词读取)、测试台
`korean` 检查读取的韩语台账、`rustfmt.toml`、`CLAUDE.md`,以及 `LICENSE` 中的 MIT 许可证。

## 文档

| 文档 | 说明 |
|---|---|
| `docs/api.md` | 固定 API:如何查找单元命令并声明其名词,如何解析和转交命令,`bmo` 自己回答哪些内容,以及 home 桥接 |
| `docs/boundary.md` | 边界所在:为引用本仓库边界文档的单元指向 `docs/api.md` 中的固定 API |
| `docs/inventory.md` | 本仓库从其拆分出来的单一仓库中继承了什么,又把什么留给各单元 |
| `docs/PERSONAL-DATA.md` | 哪些个人数据绝不进入本仓库、应改写成什么,以及读取每个受跟踪文件来检查这一点的测试台检查 |
