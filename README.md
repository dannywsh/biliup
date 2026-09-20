# biliup

本仓库是 [biliup/biliup](https://github.com/biliup/biliup) 的分支。保留上游的 B 站登录、投稿、稿件查询、评论、下载和 Web 录制服务，并增加：

- 创作中心 Web v3「视频带货 · 投稿后再添加商品」（`--post-upload-goods`）
- 评论置顶（`top-reply`）
- 会员购/票务商品链接精确识别与挂载（`goods search` / `goods attach`）
- 新版合集管理（`season list` / `season create` / `season episodes` / `season add` / `season edit` / `season remove` / `season sort`）
- 给 Agent 用的 [`skills/biliup/SKILL.md`](skills/biliup/SKILL.md)

请使用本仓库 [Releases](https://github.com/dannywsh/biliup/releases/latest) 里的 `biliup`。PyPI 的 `biliup`、`uv tool install biliup`、以及上游 [biliup/biliup](https://github.com/biliup/biliup) 的 Release 都没有上述能力。

Cookie 默认读取当前目录的 `cookies.json`，可用 `-u/--user-cookie` 覆盖。`reply`、`top-reply`、`goods attach` 以及合集的 `add`、`edit`、`remove`、`sort` 默认只预览，必须加 `--execute` 才会真正提交。

## 安装

从 [Releases](https://github.com/dannywsh/biliup/releases/latest) 下载对应平台的 zip，解压后把里面的 `biliup`（Windows 为 `biliup.exe`）放到 `PATH`。

| 平台 | 文件 |
| --- | --- |
| macOS Apple Silicon | `biliup-*-aarch64-macos.zip` |
| macOS Intel | `biliup-*-x86_64-macos.zip` |
| Linux x86_64 (glibc) | `biliup-*-x86_64-linux.zip` |
| Linux x86_64 (musl) | `biliup-*-x86_64-linux-musl.zip` |
| Linux ARM64 | `biliup-*-aarch64-linux.zip` |
| Linux ARM | `biliup-*-arm-linux.zip` |
| Windows x64 | `biliup-*-x86_64-windows.zip` |

zip 解压后是 `biliup-<版本>-<平台>/biliup`。macOS / Linux 示例：

```bash
unzip biliup-*-aarch64-macos.zip
install -m 755 biliup-*-aarch64-macos/biliup "$HOME/.local/bin/biliup"
```

若 `$HOME/.local/bin` 不在 `PATH` 里，用二进制的完整路径调用。装好后确认 `biliup --help` 有 `top-reply`、`goods`、`season`，`biliup upload --help` 有 `--post-upload-goods`、`--cover43`。

需要改代码时再从源码构建，见下方「开发」。

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
  --cover /absolute/path/cover-16x9.jpg \
  --cover43 /absolute/path/cover-4x3.jpg \
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
| `--tid-v2` | 新版分区 ID，可选；与 `--tid` 可同时设置 |
| `--cover` / `--cover43` / `--title` / `--tag` / `--desc` | 16:9 封面、4:3 首页推荐封面、标题、标签、简介 |
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

`goods` 把商品挂到**已发布**视频，不是发评论。带货评论继续用 `reply` / `top-reply`。一次挂载会同时提交两个展示位：

- 视频框下（`cmcPlaceType=1`）：标题最多 12 个字符，主图取商品详情 `main_image_url`
- 带货编辑卡（默认 `cmcPlaceType=12`）：`prefixText` / `postfixText` / `anotherName`

`goods search` 接受会员购链接、票务页 `https://show.bilibili.com/platform/detail.html?id=<数字>` 或纯数字 `itemId`。纯数字会先按会员购商品识别；会员购识别失败时会读取公开详情接口，票务页则直接读取票务详情。命令不按商品标题做模糊匹配，也不会回退到 UP 主小店搜索。

搜索结果除挂载所需的 `itemId`、`goodsName`、`sourceType`、价格和跳转链接外，还会返回 `detail` 及其摘要字段：会员购包含品牌、分类、属性、图片和摘要；票务包含城市、场馆、地址、演出日期、票价、商家、图片、简介和摘要。公开详情接口异常时，会员购仍会返回原有识别结果。票务详情用于检索展示，不能据此直接执行会员购挂载。

`goods attach` 仍只适用于可挂载的会员购商品。取得 `itemId` 后必须传 `--expected-item-id`。必须显式传 `--frame-title`（最多 12 个 Unicode 字符）。Agent 挂载流程见 [`skills/biliup/SKILL.md`](skills/biliup/SKILL.md)。

一次可以挂载多个商品：重复传入 `--query`、一次传入多个值，或用英文逗号分隔。程序会先完成所有商品识别和商品 ID 校验，再把多个商品放进一次 `createCmcTask` 请求的 `detailInfos`，生成一条评论蓝链；单条评论最多 20 个商品。第一项使用传入的前缀文案并自动追加换行；未传前缀时保持为空，中间商品使用换行后缀，保证商品逐行展示。`--expected-item-id` 可传一个值应用于全部商品，也可按商品顺序传多个值。

稿件审核中（`archive.state=-30`）已验证可以直接挂载商品，不需要等待审核通过；先 dry-run 核对商品和展示位，再加 `--execute` 提交。只有挂载接口明确因审核状态拒绝时，才每 3 分钟重新检查一次。审核拒绝（`archive.state=-2`）则停止挂载并处理 `reject_reason`。

```bash
biliup goods search 12345678
biliup goods search 'https://show.bilibili.com/platform/detail.html?id=1004629'
biliup goods attach BV1xxx --query 12345678 --expected-item-id 12345678
biliup goods attach BV1xxx \
  --query 12345678 --query 23456789 \
  --expected-item-id 12345678 --expected-item-id 23456789
biliup goods attach BV1xxx \
  --query 12345678 \
  --expected-item-id 12345678 \
  --frame-title '示例框下标题' \
  --another-name '示例展示名' \
  --postfix-text '示例后缀' \
  --execute
```

执行成功后看 `finalResult.jumpUrl`，这是后续填表用的商品链接。`--index` 选择搜索结果下标（默认 `0`）。

## 合集管理

`season` 管理 B 站新版合集（SEASON）。`list` 和 `episodes` 只读；`create`、`add`、`edit`、`remove`、`sort` 默认 dry-run，确认请求内容后再加 `--execute`。创建合集调用 B 站创作中心的 `season/add` 接口，需要提供已上传的封面 URL；成功后可用 `season list` 找到新合集及其分区。排序时必须传入目标分区中的全部视频，并按目标顺序重复传入 `--episode-id`。`edit` 会先读取目标分区的完整条目，只替换合集内标题并保留 `aid`、`cid`、排序等字段。

```bash
# 查看合集列表
biliup season list -u /absolute/path/cookies.json

# 预览并执行创建合集
biliup season create --title "示例合集" \
  --desc "示例简介" \
  --cover "https://example.com/cover.jpg" --execute

# 查看某个合集分区的视频
biliup season episodes --section-id <section_id>

# 预览并执行添加视频
biliup season add --section-id <section_id> --vid BV1xxx --vid av123456 --execute

# 预览并执行修改合集内标题
biliup season edit --section-id <section_id> --episode-id <episode_id> \
  --title "新的合集内标题" --execute

# 预览并执行移除合集内部视频
biliup season remove --episode-id <episode_id> --execute

# 按指定顺序重排全部视频
biliup season sort --season-id <season_id> --section-id <section_id> \
  --episode-id <episode_id_1> --episode-id <episode_id_2> --execute
```

合集 ID、分区 ID 和 episode ID 来自 `season list` / `season episodes` 的返回值。创建接口只负责创建合集，创建后如需加入视频，再使用 `season add`。CLI 不会自动把刚投稿的视频加入合集，也不会在未指定 `--execute` 时修改合集。创建和其他写操作都不会打印 Cookie 或 CSRF。

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
season     管理新版合集（包含标题编辑）
list       列出已投稿视频
download   下载视频
dump-flv   输出 FLV 元数据
server     启动 Web 录制服务，默认 127.0.0.1:19159
```

全局选项：`-u/--user-cookie`、`-p/--proxy`、`--rust-log`。具体参数以 `biliup <command> --help` 为准。

单独下载一场直播/视频，无需启动服务：

```bash
biliup download <URL> -o "./video/%Y-%m-%dT%H_%M_%S{title}" --split-time 1h
```

`--split-size` 与 `--split-time` 可按体积或时长自动分段，`-o` 支持 `{title}` 占位符与 strftime 时间格式。默认下载器 `stream-gears` 无需外部程序；配置 `ffmpeg` 下载器或后处理时需要本机 `ffmpeg`；YouTube、niconico 与未内置的地址则依赖 `yt-dlp` 或 `streamlink`。

### Web 录制服务

上游的录制 Web UI 仍可用。默认只监听本机；从其他设备访问时必须同时加 `--bind 0.0.0.0 --auth`：

```bash
biliup server --auth
biliup server --bind 0.0.0.0 --port 19159 --auth
```

首次打开会引导设置管理员密码，用户名固定为 `biliup`；启动后请尽快完成初始化，避免被他人抢先占用。绑定非回环地址且未开 `--auth` 会拒绝启动。经 HTTPS 反向代理时再加 `--secure-session-cookie`；直接用 HTTP 远程访问时不要加，否则浏览器会丢弃登录态。

本仓库不再发布 Docker 镜像；如需容器运行，请使用仓库内的
`docker-compose.yml` 或执行 `docker build -t biliup .` 本地构建。上游镜像
`ghcr.io/biliup/caution` 不包含本仓库的 CLI 扩展。

## Agent Skill

```bash
npx skills add dannywsh/biliup -g -y
```

只安装 `skills/biliup/SKILL.md`，不会把整个 CLI 仓库拷进 skill 目录。CLI 二进制仍从本仓库 Release 下载。

## 边录边传（sync-downloader）

把 `downloader` 设为 `sync-downloader` 后，ffmpeg 会把直播流 remux 成 Matroska 写到 stdout，按 UPOS 分片一边录一边上传，每录满 `file_size`（默认 2.5 GB，向上对齐到 10 MiB）就作为一 P 追加到同一稿件。前提：

- 本机 `PATH` 中有 `ffmpeg`；HLS 流（B 站 `bili_protocol = "hls_fmp4"`）若装有 `streamlink` 会用它拉流再交给 ffmpeg，没有则由 ffmpeg 直拉。
- 主播必须绑定上传模板，且模板对应的 cookies 文件可用；`uploader = "Noop"` 或没有模板时不会录制，日志会给出 `边录边传需要先为主播设定上传模板`。
- 上传并发固定 3 线程，不受 `threads`、`segment_time` 控制。

```toml
downloader = "sync-downloader"
uploader = "bili_web"
file_size = 2621440000          # 每 P 大小，可按上传带宽调小
# sync_save_dir = "/data/sync"  # 可选：额外保留每 P 的本地副本

[streamers."某主播"]
url = ["https://live.bilibili.com/1234"]
title = "{streamer}%Y-%m-%d 直播录像"
tid = 171
user_cookie = "cookies.json"
```

行为说明：

- 录制期间每 P 会同时写入系统临时目录（`$TMPDIR/biliup-sync/worker-<id>/`）作为兜底；预传完整且校验一致时直接 complete，否则（直播提前结束、上传落后）按实际长度从临时文件重传。投稿确认后临时文件删除，设置了 `sync_save_dir` 的副本保留并交给 `postprocessor`。
- 停止/暂停/编辑主播只会结束当前分段，已录内容仍会上传并投稿；上传或投稿失败时保留有内容的分段文件并在日志中打印路径，可手动补传。
- B 站直链约 1 小时过期。分段结束后拉不到数据时会立刻交回监控循环重新解析直链，并在同一稿件上继续追加分 P。
- 排查问题时留意 `ERROR ... 下载流程出错` 日志：会带上具体原因（cookies 文件路径、preupload 拒绝等），而不是静默退回落盘录制。

## 开发

仓库是 Rust 工作区 + 精简 Python 包 + Next.js 前端：

| 路径 | 作用 |
| --- | --- |
| `crates/biliup` | 核心库：登录、上传、评论、商品挂载、直播解析 |
| `crates/biliup-cli` | `biliup` 命令行与 Web API |
| `crates/danmaku` | 弹幕 |
| `crates/stream-gears` | Python 绑定 |
| `app/`、`public/` | Next.js Web UI；`npm run build` 产物输出到 `out/` 并由后端内嵌 |
| `biliup/` | 精简 Python 包：`python3 -m biliup` 入口 |

```bash
# 前端（cargo 构建前需要 out/）
npm i
npm run build
npm run dev          # http://localhost:3000

# CLI
cargo build --release --bin biliup
cargo test -p biliup-cli -p biliup -p danmaku
cargo run -p biliup-cli --bin biliup -- --help

# Python 入口（录制服务）
maturin dev
npm run build
python3 -m biliup
```

推送形如 `vX.Y.Z` 的标签会创建 GitHub Release；各平台二进制均以 ZIP 格式发布。

## 上游与许可

基于 [biliup/biliup](https://github.com/biliup/biliup)，许可证为 MIT。内置直播平台与弹幕能力见上游 README。

本项目仅供个人学习研究，使用产生的后果由使用者自行承担，并遵守 B 站与版权方规定。
