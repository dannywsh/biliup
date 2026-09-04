---
name: biliup
description: Use the biliup CLI to log in, upload and download Bilibili videos, run the WebUI/recorder, list archives, manage comments (comments, reply, top-reply), and attach Membership Shop or ticketing goods by mall URL, ticketing-page id, or itemId (goods search, goods attach). Use for B站会员购商品挂载、票务商品挂载、选品车、视频框下或带货编辑; not for posting ordinary comments or title-based product search.
---

# biliup

Use this skill to install and operate `biliup`. Inspect `biliup <command> --help` (or `-h`) before generating a command. Include concrete paths for cookies, configs, covers, and videos.

This repository adds `--post-upload-goods`, `top-reply`, and `goods`. Use the binary built from this source, not a generic release.

## Install

Build and install from this repository:

```bash
cargo build --release -p biliup-cli --bin biliup
install -m 755 target/release/biliup "$HOME/.local/bin/biliup"
```

If the workspace root is the parent of this repo, `cd` into `biliup` first. Confirm `biliup --help` lists `top-reply` and `goods`, `biliup upload --help` lists `--post-upload-goods`, and `biliup goods attach --help` lists `--frame-title`. If `$HOME/.local/bin` is not on `PATH`, call the binary by full path.

## Upload with post-upload goods

For the creator-center option 商业推广 → 视频带货 → 投稿后再添加商品, use `--submit web --post-upload-goods`. The flag generates a random `adorder_id` in the same range as the web page and sets `adorder_type: 2`. Use it only with `--submit web`; APP and Bcut reject it.

```bash
biliup upload \
  --submit web \
  --post-upload-goods \
  --cover /absolute/path/cover.jpg \
  --title "title" \
  --tid 65 \
  --tag "tag1,tag2" \
  /absolute/path/video.mp4
```

During review, `biliup show` may report `adorder_id: 0` and `has_porder: 0` even after a successful declaration. Confirm on the creator-center edit-detail response: nonzero `archive.adorder_id` and `archive.new_adorder_info.adorder_type == 2`.

## Comments

Cookie file defaults to `cookies.json` in the current working directory. Override with `-u/--user-cookie`.

`reply` and `top-reply` are dry-run unless `--execute` is passed. `rpid=0` posts a top-level comment. After a pin, `biliup comments <vid>` prints `top rpid=<rpid>` first. Newly posted comments may lag because of audit or cache.

```bash
biliup comments BV1xxx
biliup comments BV1xxx --sort 2 --pn 1 --ps 20
biliup reply BV1xxx 0 "comment" --execute
biliup top-reply BV1xxx <rpid> --execute
biliup top-reply BV1xxx <rpid> --unpin --execute
```

## Membership Shop goods

`goods attach` is dry-run unless `--execute` is passed. One attach writes two placements at once:

- under the player (`cmcPlaceType=1`): `--frame-title` + product `main_image_url`
- 带货编辑 card (default `cmcPlaceType=12`): `--prefix-text` / `--postfix-text` / `--another-name`

This is not a comment; use `reply` / `top-reply` for 带货评论. If the video was just uploaded and goods will be attached after review, the upload must use `--submit web --post-upload-goods`.

### Identify the product

`goods search` and `goods attach --query` accept a full `https://mall.bilibili.com/` product URL or a numeric `itemId`. They do not accept a product-name keyword. For a ticketing page `https://show.bilibili.com/platform/detail.html?id=<数字>`, the `id` query parameter is the ticketing `itemId` (for example `...?id=1004629` → `1004629`). Do not pass the `show.bilibili.com` URL to the CLI.

Numeric `itemId` is converted to a Membership Shop URL and checked exactly. Do not fall back to title search or UP 主小店 search. If the user only has a product name, ask for an itemId, mall URL, or ticketing page that contains `id`.

Once `itemId` is known, always pass `--expected-item-id`. `vid` may be av or bv; the command resolves AID internally.

```bash
biliup goods search 12345678
biliup goods search 'https://mall.bilibili.com/detail.html?itemsId=12345678'

# 票务页 https://show.bilibili.com/platform/detail.html?id=1004629
biliup goods search 1004629
```

Success output includes `itemId`, `goodsName`, `sourceType` (`5` for Membership Shop alliance), and `jumpUrl` (`mall.bilibili.com`). On identification failure, report the API output and ask the user to verify the URL or itemId.

### Review state before attach

After publish returns BV/AV, run `biliup show <vid>` and record `archive.state` / `archive.state_desc`, then dry-run. In-review drafts may be dry-run. If product, placements, and copy are correct, `--execute` is allowed. Poll every **3 minutes** only when the attach API refuses because of review state.

| `archive.state` | meaning | action |
|---|---|---|
| `0` | passed | attach (dry-run, then `--execute`) |
| `-30` | in review | dry-run is ok; success of `--execute` completes the job; if the API refuses, wait 180s and re-check |
| `-2` | rejected | stop attach; report `archive.reject_reason` |

For other non-`0` states, follow the dry-run and execute API response. If the API requires waiting, `sleep 180` between checks; do not poll faster.

```bash
biliup show BV1xxx
```

### `--frame-title` (under-player title)

Always pass `--frame-title`. If omitted, the CLI hard-truncates the display name to 12 characters, which is not a usable 带货 title.

- Count Unicode characters. Max **12**. Chinese, English, digits, spaces, and punctuation each count as 1.
- The CLI rejects more than 12 characters. Count before sending.
- If the user's title is longer, compress to 12 first, preview, and tell the user the short title used.

Content must identify the attached SKU:

- Keep **IP/character + category** (or the accessory's own category).
- Drop English brand prefixes, SKU codes, and long modifiers. Keep 礼盒 / 配件 / 特典 only when that is the current SKU.
- Accessory SKU → accessory wording; main SKU → not accessory wording.
- Do not invent selling points absent from the product name or the user's notes.

On dry-run, check `attach.cmcInfos`: first item `cmcPlaceType=1`, `title` ≤ 12, `imageUrl` non-empty; second item `cmcPlaceType=12`.

### Attach workflow

1. `goods search` with mall URL or itemId; confirm identity.
2. `biliup show <vid>` for review state; dry-run is allowed during review; poll every 3 minutes only if the API refuses attach.
3. Write `--frame-title` ≤ 12 characters; confirm product, video, and copy with the user.
4. `goods attach` without `--execute` to preview; then `--execute`. Items already in the selection cart skip the add-to-cart step.
5. Require `code=0` and each `resCode=0`. `finalResult.jumpUrl` is the product URL for later forms. On failure, report the API output and re-read product/archive state; do not guess-retry.

```bash
biliup goods search 12345678

biliup show BV1xxx

biliup goods attach BV1xxx \
  --query 12345678 \
  --expected-item-id 12345678 \
  --frame-title '示例框下标题' \
  --another-name '示例展示名' \
  --postfix-text '示例后缀'

biliup goods attach BV1xxx \
  --query 12345678 \
  --expected-item-id 12345678 \
  --frame-title '示例框下标题' \
  --another-name '示例展示名' \
  --postfix-text '示例后缀' \
  --execute
```

## Server

```bash
biliup server --auth
biliup server --bind 0.0.0.0 --port 19159 --auth
biliup server --config config.toml
nohup biliup server --auth &
```

## Commands

```text
login
renew
upload
append
show
comments
reply
top-reply
goods
dump-flv
download
server
list
```
