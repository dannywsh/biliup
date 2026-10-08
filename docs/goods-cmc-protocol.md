# 商品挂载资源位

依据 2026-10-08 下载的 Bilibili 官方前端业务脚本。挂载类型由用户选择的资源位决定，商品只提供 ID、展示文本和图片，不能用某个商品的一次请求推断所有商品的类型。

| 资源位 | `fromType` | `cmcPlaceType` | 前端数量上限 |
| --- | --- | --- | --- |
| 评论蓝链 | 3 | 12 | 每条评论 20 个商品 |
| 视频框下 | 7 | 1 | 5 个商品，新增时扣除已有数量 |

官方内容管理脚本直接定义 `COMMENT=3`、`UNDER_VIDEO=7` 的任务来源，以及 `COMMENT=12`、`UNDER_VIDEO=1` 的展示位。资源位编辑组件根据 `types` 中的 `CommentLinks` 或 `UnderVideo` 构造对应任务；新增评论组件直接使用评论常量。这些字段不是商品接口返回后透传的值。

## biliup 多商品评论

`goods attach` 传多个商品时，通过 `createCmcTask` 提交一个评论任务：

```json
{
  "cmcInfos": [{
    "avId": "123456789",
    "fromType": 3,
    "masTaskId": 0,
    "detailInfos": [{
      "cmcPlaceType": 12,
      "title": "示例商品",
      "itemId": "10000001",
      "anotherName": "示例商品",
      "imageUrl": "https://example.com/synthetic.png",
      "prefixText": "",
      "postfixText": ""
    }]
  }],
  "requestFrom": 109
}
```

示例全部为合成数据。每件商品对应一个详情，展示名来自 `--another-name` 或商品名称，不采用视频框下的短标题。文本按前端默认规则截取为最多 32 个 UTF-16 单元，且不拆分 Unicode 字符；前 9 件商品附图，其余仍保留商品链接。共享前文案只放在第一件商品之前，共享后文案只放在最后一件商品之后，商品之间换行。

多商品的 `--place-type` 必须为 12；`--frame-title` 仅用于单商品视频框下展示位。所有商品和请求体在选品车写入前完成预检。单商品使用原有 `batch/commit` 请求，同时提交框下和评论展示位。

## 业务失败

外层 `code=0` 只能说明请求被接口处理。多商品创建任务还需要检查 `data.failCnt`，失败提示来自 `data.failTips`；若返回商品级 `resCode`，也必须为零。biliup 对这些失败返回错误并保留接口响应，不自动重试，避免重复创建评论。选品车接口业务失败时停止后续挂载。

## 官方源码入口

创作中心 `mutual_up` 路由的 [chunk 188](https://s1.hdslb.com/bfs/static/studio/creativecenter-platform-next/static/js/async/js/creativecenter-v3.188.087b8a68.js) 嵌入 `cm.bilibili.com/quests/#/task`。带货页面入口加载内容管理业务脚本：

- [manage-DChbySzk.js](https://s1.hdslb.com/bfs/static/cm/quests/assets/manage-DChbySzk.js)：类型常量、资源位选择、请求构造与 `failCnt` 处理。
- [sycpb.cpm.tavern-platform-1OInjWWX.js](https://s1.hdslb.com/bfs/static/cm/quests/assets/sycpb.cpm.tavern-platform-1OInjWWX.js)：`createCmcTask` 接口包装。
- [index-CCFb32J2.js](https://s1.hdslb.com/bfs/static/cm/quests/assets/index-CCFb32J2.js)：将旧接口路径映射至 `mall.bilibili.com/mall-cbp/web/task/op/createCmcTask`。

上述文件名带构建哈希，后续前端发布可能更换文件名。业务脚本本次 SHA-256 为 `2443b51a126e491da3ffc2d9691db4ed5b0228e46c8ac31f0698ac1c0ab872d4`。
