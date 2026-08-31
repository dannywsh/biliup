---
name: biliup
description: Use the biliup CLI to log in, upload and download Bilibili videos, run the WebUI/recorder, list archives, manage comments (comments, reply, top-reply), and attach Membership Shop goods (goods search, goods attach).
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

If the workspace root is the parent of this repo, `cd` into `biliup` first. Confirm `biliup --help` lists `top-reply` and `goods`, and `biliup upload --help` lists `--post-upload-goods`. If `$HOME/.local/bin` is not on `PATH`, call the binary by full path.

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

`goods attach` is dry-run unless `--execute` is passed. It searches Membership Shop items (`sourceType=5`, sellable, `mall.bilibili.com` jump URL), adds the item to the selection cart when needed, and attaches it to a published video in two placements at once: under the player (`cmcPlaceType=1`) and the 带货编辑 card (default `cmcPlaceType=12`). This is not a comment; use `reply` / `top-reply` for 带货评论.

If the user gives an `itemId`, pass `--expected-item-id` so a similar search hit cannot be attached. `vid` may be av or bv; the command resolves AID internally. `finalResult.jumpUrl` is the product link to reuse later. `--frame-title` is the under-player title and must be at most 12 characters; if omitted, the display name is truncated.

```bash
biliup goods search '示例商品'
biliup goods attach BV1xxx --query '示例商品' --expected-item-id 12345678
biliup goods attach BV1xxx \
  --query '示例商品' \
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
