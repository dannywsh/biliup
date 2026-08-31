# biliup

本仓库是 [biliup/biliup](https://github.com/biliup/biliup) 的分支。保留上游的 B 站登录、投稿、稿件查询、评论、下载和 Web 录制服务，并增加：

- 创作中心 Web v3「视频带货 · 投稿后再添加商品」（`--post-upload-goods`）
- 评论置顶（`top-reply`）
- 会员购联盟 / UP 主小店商品搜索与挂载（`goods search` / `goods attach`）
- 给 Agent 用的 [`SKILL.md`](SKILL.md)

**请使用本仓库构建或发布的 `biliup`。** PyPI 上的 `biliup`、上游 GitHub Release，以及 `uv tool install biliup` 都不包含上述能力。安装后用 `biliup --help` 确认存在 `top-reply`、`goods`；用 `biliup upload --help` 确认存在 `--post-upload-goods`。

Cookie 默认读取当前目录的 `cookies.json`，可用 `-u/--user-cookie` 覆盖。`reply`、`top-reply`、`goods attach` 默认只预览，必须加 `--execute` 才会真正提交。

## 安装

### 下载 Release

从 [Releases](https://github.com/dannywsh/biliup/releases/latest) 下载对应平台的 `biliup` 二进制，放到 `PATH` 中即可。

### 从源码构建

需要 Rust 工具链。若还要带 Web UI 静态资源，先构建前端：

```bash
npm i
npm run build
cargo build --release -p biliup-cli --bin biliup
install -m 755 target/release/biliup "$HOME/.local/bin/biliup"
```

只做命令行投稿、评论、商品挂载时，也可以跳过 `npm`，直接 `cargo build --release -p biliup-cli --bin biliup`。若 `$HOME/.local/bin` 不在 `PATH` 里，用二进制的完整路径调用。

## 登录

```bash
biliup login
biliup renew
```

`login` 支持账号密码、短信、扫码、浏览器、网页 Cookie。登录后 cookie 与 token 写入 `cookies.json`（或 `-u` 指定的文件），后续命令都会用这份凭据。

## 投稿

默认走 APP 投稿接口。声明「商业推广 → 视频带货 → 投稿后再添加商品」时，必须用 `--submit web --post-upload-goods`：

```bash
biliup upload \
  --submit web \
  --post-upload-goods \
  --cover /absolute/path/cover.jpg \
  --title "视频标题" \
  --tid 65 \
  --tag "标签1,标签2" \
  /absolute/path/video.mp4
```

`--post-upload-goods` 会按创作中心网页规则生成 `adorder_id`，并提交 `adorder_type: 2`。APP 和必剪接口会拒绝该参数。

审核期间 `biliup show` 可能仍显示 `adorder_id: 0`、`has_porder: 0`。以创作中心编辑详情为准：`archive.adorder_id` 非 0，且 `archive.new_adorder_info.adorder_type == 2`。

常用参数：

| 参数 | 说明 |
| --- | --- |
| `--submit` | 投稿接口：`app`（默认）、`web`、`b-cut-android` |
| `--tid` | 分区，默认 `171` |
| `--cover` / `--title` / `--tag` / `--desc` | 封面、标题、标签、简介 |
| `--copyright` | `1` 自制（默认），`2` 转载 |
| `--dtime` | 定时发布，10 位时间戳，且距提交须大于 4 小时 |
| `-l/--line` | 上传线路 |
| `-c/--config` | 用配置文件投稿，此时不需要视频路径参数 |

追加分 P、查看稿件：

```bash
biliup append -v BV1xxx video.mp4
biliup show BV1xxx
biliup list --pubed
```

## 评论

`rpid=0` 表示发表顶级评论。置顶成功后，`biliup comments` 会先打印 `top rpid=<rpid>`。新评论可能因审核或缓存暂时看不到。

```bash
biliup comments BV1xxx
biliup comments BV1xxx --sort 2 --pn 1 --ps 20
biliup reply BV1xxx 0 "评论内容" --execute
biliup top-reply BV1xxx <rpid> --execute
biliup top-reply BV1xxx <rpid> --unpin --execute
```

`--sort`：`0` 按时间（默认），`2` 按热度。

## 商品挂载

`goods` 把商品挂到**已发布**视频的带货编辑位，不是发评论。带货评论继续用 `reply` / `top-reply`。

搜索顺序：先查可售会员购联盟商品（`sourceType=5`），没有命中再查 UP 主小店（`sourceType=8`）。两类都要求可售，且跳转 `mall.bilibili.com`。用户给出 `itemId` 时传入 `--expected-item-id`，避免检索词相似导致挂错。稿件可用 av 或 bv，命令内部会转成 AID。已在选品车的商品会跳过加入步骤。

```bash
biliup goods search '示例商品'
biliup goods attach BV1xxx --query '示例商品' --expected-item-id 12345678
biliup goods attach BV1xxx \
  --query '示例商品' \
  --expected-item-id 12345678 \
  --another-name '示例展示名' \
  --postfix-text '示例后缀' \
  --execute
```

执行成功后看 `finalResult.jumpUrl`，这是后续填表用的商品链接。`--index` 选择搜索结果下标（默认 `0`），`--place-type` 是带货展示位（默认 `12`）。

## 其他命令

```text
login      登录并保存 cookies.json
renew      刷新登录信息
upload     上传并投稿
append     向已有稿件追加分 P
show       打印稿件详情
comments   查看评论
reply      发表或回复评论（默认 dry-run）
top-reply  置顶或取消置顶（默认 dry-run）
goods      搜索商品，或挂载到已发布视频
list       列出已投稿视频
download   下载视频
dump-flv   输出 FLV 元数据
server     启动 Web 录制服务，默认 127.0.0.1:19159
```

全局选项：`-u/--user-cookie`、`-p/--proxy`、`--rust-log`。具体参数以 `biliup <command> --help` 为准。

### Web 录制服务

上游的录制 Web UI 仍可用。默认只监听本机；从其他设备访问时必须同时加 `--bind 0.0.0.0 --auth`：

```bash
biliup server --auth
biliup server --bind 0.0.0.0 --port 19159 --auth
```

首次打开会引导设置管理员密码，用户名固定为 `biliup`。绑定非回环地址且未开 `--auth` 会拒绝启动。经 HTTPS 反向代理时再加 `--secure-session-cookie`；直接用 HTTP 远程访问时不要加，否则浏览器会丢弃登录态。

本仓库不再发布 Docker 镜像；如需容器运行，请使用仓库内的
`docker-compose.yml` 或执行 `docker build -t biliup .` 本地构建。上游镜像
`ghcr.io/biliup/caution` 不包含本仓库的 CLI 扩展。

## Agent Skill

把仓库里的 `SKILL.md` 复制到 Agent 的 skill 目录，目录名使用 `biliup`：

```bash
mkdir -p ~/.agents/skills/biliup
cp SKILL.md ~/.agents/skills/biliup/SKILL.md
```

Grok 对应路径为 `~/.grok/skills/biliup/SKILL.md`。

## 开发

仓库是 Rust 工作区 + 精简 Python 包 + Next.js 前端：

| 路径 | 作用 |
| --- | --- |
| `crates/biliup` | 核心库：登录、上传、评论、商品挂载、直播解析 |
| `crates/biliup-cli` | `biliup` 命令行与 Web API |
| `crates/danmaku` | 弹幕 |
| `crates/stream-gears` | Python 绑定 |
| `app/` | Next.js Web UI |

```bash
# 前端
npm i
npm run dev          # http://localhost:3000

# CLI
cargo build --release --bin biliup
cargo test -p biliup-cli -p biliup
cargo run -p biliup-cli --bin biliup -- --help

# Python 入口（录制服务）
maturin dev
npm run build
python3 -m biliup
```

## 上游与许可

基于 [biliup/biliup](https://github.com/biliup/biliup)，许可证为 MIT。直播下载依赖 `streamlink`、`yt-dlp` 等上游能力。

本项目仅供个人学习研究，使用产生的后果由使用者自行承担，并遵守 B 站与版权方规定。
