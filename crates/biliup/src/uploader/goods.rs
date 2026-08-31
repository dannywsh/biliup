use super::bilibili::{BiliBili, Vid};
use crate::error::{Kind, Result};
use serde_json::{Value, json};

const MEMBERSHIP_ALLIANCE_SOURCE_TYPE: i64 = 5;
const UP_STORE_SOURCE_TYPE: i64 = 8;
const SEARCH_SOURCE_TYPES: [i64; 2] = [MEMBERSHIP_ALLIANCE_SOURCE_TYPE, UP_STORE_SOURCE_TYPE];
const SEARCH_URL: &str = "https://mall.bilibili.com/mall-cbp/web/shop_goods/search";
const GOODS_DETAIL_URL: &str = "https://mall.bilibili.com/mall-cbp/web/shop_goods/id";
const ADD_TO_CART_URL: &str = "https://mall.bilibili.com/mall-cbp/web/selectionCart/item/add";
const ATTACH_URL: &str = "https://mall.bilibili.com/mall-cbp/web/task/op/batch/commit";
const SEARCH_PAGE: u32 = 1;
const SEARCH_SIZE: u32 = 20;
/// 视频框下商品卡展示位。
pub const UNDER_VIDEO_PLACE_TYPE: u32 = 1;
/// 带货编辑默认展示位。
pub const DEFAULT_CARD_PLACE_TYPE: u32 = 12;
/// 视频框下标题最大字符数。
pub const UNDER_VIDEO_TITLE_MAX_CHARS: usize = 12;
const UNDER_VIDEO_STYLE: u8 = 1;

/// 构造挂载计划所需的检索、展示位和文案参数。
#[derive(Debug, Clone, Copy)]
pub struct GoodsAttachOptions<'a> {
    /// 商品检索词。
    pub query: &'a str,
    /// 稿件 av 或 bv。
    pub vid: &'a Vid,
    /// 搜索结果下标。
    pub index: usize,
    /// 带货编辑展示位，默认 12；会与视频框下（1）一并提交。
    pub place_type: u32,
    /// 带货卡片前文案。
    pub prefix_text: &'a str,
    /// 带货卡片后文案。
    pub postfix_text: &'a str,
    /// 带货卡片展示别名；为空时使用商品原名。
    pub another_name: &'a str,
    /// 视频框下标题；为空时从展示名截取，最多 12 个字符。
    pub frame_title: Option<&'a str>,
    /// 可选的商品 ID 白名单。
    pub expected_item_id: Option<&'a str>,
}

/// 会员购挂载预览结果。
///
/// `item` 是通过会员购校验的搜索结果；`cart_payload` / `attach_payload` 是将要提交的请求体。
#[derive(Debug, Clone)]
pub struct GoodsAttachPlan {
    pub item: Value,
    pub cart_payload: Value,
    pub attach_payload: Value,
}

impl GoodsAttachPlan {
    /// 判断该商品是否尚未加入选品车。
    ///
    /// 输入：无。返回：`inSelectionCarState == 0` 时为 `true`。
    pub fn needs_add_to_cart(&self) -> bool {
        json_i64(self.item.get("inSelectionCarState")).is_some_and(|state| state == 0)
    }

    /// 整理挂载工作流的最终输出。
    ///
    /// 输入：`mode` 为 `preview` 或 `executed`，以及选品车、挂载接口结果。
    /// 返回：包含填表所需 `jumpUrl` 的 JSON 对象。
    pub fn final_result(
        &self,
        mode: &str,
        cart_result: Value,
        attach_result: Value,
    ) -> Result<Value> {
        Ok(json!({
            "finalResult": {
                "mode": mode,
                "itemId": required_item_id(&self.item)?,
                "goodsName": required_string(&self.item, "goodsName")?,
                "jumpUrl": required_string(&self.item, "jumpUrl")?,
                "cartResult": cart_result,
                "attachResult": attach_result,
            }
        }))
    }
}

/// 按商品来源过滤可售的 B 站商城商品。
///
/// 输入：搜索接口 `data.items` 和目标 `source_type`。返回：来源匹配、可售且跳转 B 站商城域名的商品。
pub fn filter_sellable_mall_goods(items: &[Value], source_type: i64) -> Vec<Value> {
    items
        .iter()
        .filter(|item| is_sellable_mall_goods(item, source_type))
        .cloned()
        .collect()
}

/// 从商品对象提取搜索命令需要展示的字段。
///
/// 输入：完整商品 JSON。返回：含 `itemId`、名称、价格和跳转链接的精简对象。
pub fn summarize_goods_item(item: &Value, index: usize) -> Value {
    json!({
        "index": index,
        "itemId": item.get("itemId"),
        "goodsName": item.get("goodsName"),
        "sourceType": item.get("sourceType"),
        "price": item.get("price"),
        "commissionFee": item.get("commissionFee"),
        "inSelectionCarState": item.get("inSelectionCarState"),
        "jumpUrl": item.get("jumpUrl"),
    })
}

/// 由搜索结果构造前端同构的选品车请求体。
///
/// 输入：`item` 为搜索结果对象，`page` 为搜索页码，`index` 为结果下标。
/// 返回：加入选品车接口的 JSON body。
pub fn build_cart_payload(item: &Value, page: u32, index: usize) -> Result<Value> {
    let mut cart_item = item.clone();
    let object = cart_item
        .as_object_mut()
        .ok_or_else(|| Kind::Custom("商品数据必须是 JSON 对象".to_string()))?;
    object.insert(
        "income".to_string(),
        item.get("commissionFee").cloned().unwrap_or(json!(0)),
    );
    object.insert("position".to_string(), json!(format!("{page}-{index}")));
    Ok(json!({
        "goods": [cart_item],
        "operateSource": 4,
        "bizExtraInfo": "",
        "fromType": 18,
    }))
}

/// 截取指定数量的 Unicode 字符。
///
/// 输入：`text` 为原文，`max_chars` 为最大字符数。返回：截断后的字符串。
pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

/// 生成视频框下标题。
///
/// 输入：`display_name` 为商品展示名，`frame_title` 为可选显式标题。
/// 返回：不超过 12 个字符的标题；显式标题超长或结果为空时返回错误。
pub fn under_video_title(display_name: &str, frame_title: Option<&str>) -> Result<String> {
    match frame_title.map(str::trim).filter(|text| !text.is_empty()) {
        Some(explicit) => {
            let count = explicit.chars().count();
            if count > UNDER_VIDEO_TITLE_MAX_CHARS {
                return Err(Kind::Custom(format!(
                    "视频框下标题最多 {UNDER_VIDEO_TITLE_MAX_CHARS} 个字符，当前为 {count}。"
                )));
            }
            Ok(explicit.to_string())
        }
        None => {
            let title = truncate_chars(display_name.trim(), UNDER_VIDEO_TITLE_MAX_CHARS);
            if title.is_empty() {
                return Err(Kind::Custom("视频框下标题不能为空".to_string()));
            }
            Ok(title)
        }
    }
}

/// 从商品详情接口响应提取主图。
///
/// 输入：`response` 为 `shop_goods/id` 的 JSON。返回：`data.main_image_url`。
pub fn parse_main_image_url(response: &Value) -> Result<String> {
    response
        .pointer("/data/main_image_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .ok_or_else(|| Kind::Custom("商品详情缺少 main_image_url".to_string()))
}

/// 构造会员购商品挂载请求体。
///
/// 输入：商品 ID、视频 AID、带货编辑展示位、卡片文案、视频框下标题和主图。
/// 返回：同时包含视频框下（`cmcPlaceType=1`）和带货编辑卡的 JSON body；带货展示位为 1 时不重复提交。
pub fn build_attach_payload(
    item_id: &str,
    aid: u64,
    place_type: u32,
    prefix_text: &str,
    postfix_text: &str,
    another_name: &str,
    frame_title: &str,
    image_url: &str,
) -> Value {
    let mut cmc_infos = vec![json!({
        "cmcPlaceType": UNDER_VIDEO_PLACE_TYPE,
        "title": frame_title,
        "imageUrl": image_url,
        "style": UNDER_VIDEO_STYLE,
        "masTaskId": "",
    })];
    if place_type != UNDER_VIDEO_PLACE_TYPE {
        cmc_infos.push(json!({
            "cmcPlaceType": place_type,
            "prefixText": prefix_text,
            "postfixText": postfix_text,
            "anotherName": another_name,
        }));
    }
    json!({
        "itemId": item_id,
        "videoInfos": [{"avId": aid.to_string()}],
        "cmcInfos": cmc_infos,
    })
}

/// 校验选中商品 ID 是否等于用户指定值。
///
/// 输入：`item` 为选中商品，`expected_item_id` 为可选白名单。
/// 返回：匹配或不需要校验时 `Ok(())`；不一致时返回错误以阻止写操作。
pub fn validate_expected_item_id(item: &Value, expected_item_id: Option<&str>) -> Result<()> {
    let Some(expected) = expected_item_id else {
        return Ok(());
    };
    let actual = required_item_id(item)?;
    if actual == expected {
        return Ok(());
    }
    let name = item.get("goodsName").and_then(Value::as_str).unwrap_or("");
    Err(Kind::Custom(format!(
        "商品 ID 校验失败：期望 {expected}，搜索结果为 {actual}（{name}）。"
    )))
}

/// 收集 JSON 中所有非 0 的 `resCode`。
///
/// 输入：接口响应 JSON。返回：失败项的可读描述列表。
pub fn collect_failed_res_codes(value: &Value) -> Vec<String> {
    let mut failures = Vec::new();
    collect_failed_res_codes_into(value, &mut failures);
    failures
}

fn collect_failed_res_codes_into(value: &Value, failures: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(code) = map.get("resCode")
                && json_i64(Some(code)) != Some(0)
            {
                failures.push(format!("resCode={code}"));
            }
            for nested in map.values() {
                collect_failed_res_codes_into(nested, failures);
            }
        }
        Value::Array(items) => {
            for nested in items {
                collect_failed_res_codes_into(nested, failures);
            }
        }
        _ => {}
    }
}

/// 判断商品是否为指定来源的可挂载商城商品。
///
/// 输入：商城搜索接口返回的单个商品对象和预期来源。返回：来源匹配、可售与商城跳转要求时为 `true`。
fn is_sellable_mall_goods(item: &Value, source_type: i64) -> bool {
    json_i64(item.get("sourceType")) == Some(source_type)
        && item.get("goodsStatus") == Some(&Value::Bool(true))
        && item
            .get("jumpUrl")
            .and_then(Value::as_str)
            .is_some_and(|url| url.contains("mall.bilibili.com"))
}

fn json_i64(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn required_item_id(item: &Value) -> Result<String> {
    match item.get("itemId") {
        Some(Value::String(text)) if !text.is_empty() => Ok(text.clone()),
        Some(Value::Number(number)) => Ok(number.to_string()),
        _ => Err(Kind::Custom("商品缺少 itemId".to_string())),
    }
}

fn required_string(item: &Value, field: &str) -> Result<String> {
    item.get(field)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| Kind::Custom(format!("商品缺少 {field}")))
}

fn search_items(response: &Value) -> Result<Vec<Value>> {
    match response.pointer("/data/items") {
        Some(Value::Array(items)) => Ok(items.clone()),
        Some(_) => Err(Kind::Custom("搜索接口 items 必须是数组".to_string())),
        None => Ok(Vec::new()),
    }
}

impl BiliBili {
    /// 为会员购带货请求补齐创作中心同源头。
    ///
    /// 输入：`request` 为待发送请求。返回：带 Origin、Referer 和 csrf 头的请求。
    fn with_mall_headers(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder> {
        let csrf = self.get_csrf()?;
        Ok(request
            .header("Origin", "https://member.bilibili.com")
            .header("Referer", "https://member.bilibili.com/")
            .header("Accept", "application/json, text/plain, */*")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("csrf-token", csrf)
            .header("csrf-jct", csrf))
    }

    /// 发送会员购请求并校验 `code=0`。
    ///
    /// 输入：已构造的 `request`。返回：完整 JSON；HTTP 或业务码失败时带上接口信息。
    async fn send_mall_request(&self, request: reqwest::RequestBuilder) -> Result<Value> {
        let response = request.send().await?;
        let status = response.status();
        let payload: Value = response
            .json()
            .await
            .map_err(|error| Kind::Custom(format!("接口请求失败：HTTP {status} {error}")))?;
        let code = json_i64(payload.get("code")).unwrap_or(-1);
        if code != 0 {
            let message = payload.get("message").and_then(Value::as_str).unwrap_or("");
            return Err(Kind::Custom(format!(
                "接口返回失败：code={code} message={message}"
            )));
        }
        Ok(payload)
    }

    /// 向会员购带货接口发送 JSON POST。
    ///
    /// 输入：`url` 为接口地址，`body` 为 JSON 请求体。
    /// 返回：`code=0` 的完整响应；失败时带上接口 `message`。
    async fn mall_json_post(&self, url: &str, body: &Value) -> Result<Value> {
        let request = self.with_mall_headers(self.client.post(url).json(body))?;
        self.send_mall_request(request).await
    }

    /// 向会员购带货接口发送 GET。
    ///
    /// 输入：`url` 为接口地址，`query` 为查询参数。返回：`code=0` 的完整响应。
    async fn mall_json_get(&self, url: &str, query: &[(&str, &str)]) -> Result<Value> {
        let request = self.with_mall_headers(self.client.get(url).query(query))?;
        self.send_mall_request(request).await
    }

    /// 按商品 ID 拉取详情主图。
    ///
    /// 输入：`item_id` 为商城商品 ID。返回：`data.main_image_url`。
    async fn fetch_main_image_url(&self, item_id: &str) -> Result<String> {
        let response = self
            .mall_json_get(GOODS_DETAIL_URL, &[("shop_goods_id", item_id)])
            .await?;
        parse_main_image_url(&response)
    }

    /// 按指定来源搜索可售商品。
    ///
    /// 输入：`query` 为商品检索词，`source_type` 为会员购联盟或 UP 主小店来源。
    /// 返回：该来源下可售的商城商品候选列表。
    async fn search_goods_by_source_type(
        &self,
        query: &str,
        source_type: i64,
    ) -> Result<Vec<Value>> {
        let response = self
            .mall_json_post(
                SEARCH_URL,
                &json!({
                    "cmcFirstCatNames": "",
                    "goodsName": query,
                    "query": query,
                    "page": SEARCH_PAGE,
                    "size": SEARCH_SIZE,
                    "sourceTypes": source_type.to_string(),
                    "sortType": 6,
                }),
            )
            .await?;
        Ok(filter_sellable_mall_goods(
            &search_items(&response)?,
            source_type,
        ))
    }

    /// 按优先级搜索可售商品。
    ///
    /// 输入：`query` 为商品检索词。返回：优先返回会员购联盟（`sourceType=5`）候选；为空时回退至 UP 主小店（`sourceType=8`）。
    pub async fn search_goods(&self, query: &str) -> Result<Vec<Value>> {
        let query = query.trim();
        if query.is_empty() {
            return Err(Kind::Custom("检索词不能为空".to_string()));
        }
        for source_type in SEARCH_SOURCE_TYPES {
            let candidates = self.search_goods_by_source_type(query, source_type).await?;
            if !candidates.is_empty() {
                return Ok(candidates);
            }
        }
        Ok(Vec::new())
    }

    /// 预览商品挂载：搜索、校验商品、拉取主图并构造请求体，不发起写操作。
    ///
    /// 输入：`options` 含检索词、稿件、展示位、卡片文案、可选视频框下标题和商品 ID 白名单。
    /// 返回：可供确认或随后执行的挂载计划，请求体同时包含视频框下和带货编辑卡。
    pub async fn plan_goods_attach(
        &self,
        options: GoodsAttachOptions<'_>,
    ) -> Result<GoodsAttachPlan> {
        let candidates = self.search_goods(options.query).await?;
        if candidates.is_empty() {
            return Err(Kind::Custom(
                "未找到可售会员购联盟或 UP 主小店商品；请调整检索词。".to_string(),
            ));
        }
        let item = candidates.get(options.index).cloned().ok_or_else(|| {
            Kind::Custom(format!(
                "候选下标 {} 超出范围，共 {} 个候选。",
                options.index,
                candidates.len()
            ))
        })?;
        validate_expected_item_id(&item, options.expected_item_id)?;
        let item_id = required_item_id(&item)?;
        let display_name = if options.another_name.trim().is_empty() {
            required_string(&item, "goodsName")?
        } else {
            options.another_name.to_string()
        };
        let frame_title = under_video_title(&display_name, options.frame_title)?;
        let image_url = self.fetch_main_image_url(&item_id).await?;
        let aid = self.aid_from_vid(options.vid).await?;
        Ok(GoodsAttachPlan {
            cart_payload: build_cart_payload(&item, SEARCH_PAGE, options.index)?,
            attach_payload: build_attach_payload(
                &item_id,
                aid,
                options.place_type,
                options.prefix_text,
                options.postfix_text,
                &display_name,
                &frame_title,
                &image_url,
            ),
            item,
        })
    }

    /// 执行选品车写入和视频挂载。
    ///
    /// 输入：`plan` 为预览阶段生成的挂载计划。
    /// 返回：`(cart_result, attach_result)`；已在选品车时跳过加入步骤。
    pub async fn execute_goods_attach(&self, plan: &GoodsAttachPlan) -> Result<(Value, Value)> {
        let cart_result = if plan.needs_add_to_cart() {
            self.mall_json_post(ADD_TO_CART_URL, &plan.cart_payload)
                .await?
        } else {
            json!("already_in_selection_cart")
        };
        let attach_result = self
            .mall_json_post(ATTACH_URL, &plan.attach_payload)
            .await?;
        let failures = collect_failed_res_codes(&attach_result);
        if !failures.is_empty() {
            return Err(Kind::Custom(format!(
                "挂载接口返回失败：{}",
                failures.join(", ")
            )));
        }
        Ok((cart_result, attach_result))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_CARD_PLACE_TYPE, GoodsAttachPlan, UNDER_VIDEO_PLACE_TYPE,
        UNDER_VIDEO_TITLE_MAX_CHARS, build_attach_payload, build_cart_payload,
        collect_failed_res_codes, filter_sellable_mall_goods, parse_main_image_url,
        summarize_goods_item, truncate_chars, under_video_title, validate_expected_item_id,
    };
    use serde_json::json;

    fn up_store_item() -> serde_json::Value {
        json!({
            "itemId": 12345678,
            "goodsName": "示例 UP 主小店商品",
            "sourceType": 8,
            "goodsStatus": true,
            "price": 99,
            "commissionFee": 12,
            "inSelectionCarState": 0,
            "jumpUrl": "https://mall.bilibili.com/detail.html?itemId=12345678"
        })
    }

    fn membership_alliance_item() -> serde_json::Value {
        json!({
            "itemId": 87654321,
            "goodsName": "示例会员购联盟商品",
            "sourceType": 5,
            "goodsStatus": true,
            "price": 99,
            "commissionFee": 12,
            "inSelectionCarState": 0,
            "jumpUrl": "https://mall.bilibili.com/detail.html?itemId=87654321"
        })
    }

    #[test]
    fn source_filter_keeps_only_matching_sellable_mall_items() {
        let taobao = json!({
            "itemId": "1",
            "sourceType": 1,
            "goodsStatus": true,
            "jumpUrl": "https://item.taobao.com/item.htm"
        });
        let unavailable = json!({
            "itemId": "2",
            "sourceType": 8,
            "goodsStatus": false,
            "jumpUrl": "https://mall.bilibili.com/detail.html?itemId=2"
        });
        let filtered = filter_sellable_mall_goods(&[up_store_item(), taobao, unavailable], 8);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["itemId"], json!(12345678));
    }

    #[test]
    fn source_filters_keep_membership_alliance_and_up_store_separate() {
        let items = vec![membership_alliance_item(), up_store_item()];
        let membership = filter_sellable_mall_goods(&items, 5);
        let up_store = filter_sellable_mall_goods(&items, 8);
        assert_eq!(membership[0]["itemId"], json!(87654321));
        assert_eq!(up_store[0]["itemId"], json!(12345678));
    }

    #[test]
    fn cart_payload_copies_search_item_and_maps_commission() {
        let payload = build_cart_payload(&up_store_item(), 1, 0).unwrap();
        assert_eq!(payload["operateSource"], json!(4));
        assert_eq!(payload["fromType"], json!(18));
        assert_eq!(payload["goods"][0]["income"], json!(12));
        assert_eq!(payload["goods"][0]["position"], json!("1-0"));
        assert_eq!(payload["goods"][0]["itemId"], json!(12345678));
    }

    #[test]
    fn attach_payload_includes_under_video_and_card_placements() {
        let payload = build_attach_payload(
            "12345678",
            123456789,
            DEFAULT_CARD_PLACE_TYPE,
            "",
            "示例后缀",
            "示例展示名",
            "示例框下标题",
            "https://example.com/cover.png",
        );
        assert_eq!(payload["itemId"], json!("12345678"));
        assert_eq!(payload["videoInfos"][0]["avId"], json!("123456789"));
        assert_eq!(payload["cmcInfos"].as_array().unwrap().len(), 2);
        assert_eq!(
            payload["cmcInfos"][0]["cmcPlaceType"],
            json!(UNDER_VIDEO_PLACE_TYPE)
        );
        assert_eq!(payload["cmcInfos"][0]["title"], json!("示例框下标题"));
        assert_eq!(
            payload["cmcInfos"][0]["imageUrl"],
            json!("https://example.com/cover.png")
        );
        assert_eq!(payload["cmcInfos"][0]["style"], json!(1));
        assert_eq!(payload["cmcInfos"][0]["masTaskId"], json!(""));
        assert_eq!(
            payload["cmcInfos"][1]["cmcPlaceType"],
            json!(DEFAULT_CARD_PLACE_TYPE)
        );
        assert_eq!(payload["cmcInfos"][1]["postfixText"], json!("示例后缀"));
        assert_eq!(payload["cmcInfos"][1]["anotherName"], json!("示例展示名"));
    }

    #[test]
    fn attach_payload_skips_duplicate_under_video_card() {
        let payload = build_attach_payload(
            "12345678",
            1,
            UNDER_VIDEO_PLACE_TYPE,
            "",
            "",
            "示例展示名",
            "示例框下标题",
            "https://example.com/cover.png",
        );
        assert_eq!(payload["cmcInfos"].as_array().unwrap().len(), 1);
        assert_eq!(
            payload["cmcInfos"][0]["cmcPlaceType"],
            json!(UNDER_VIDEO_PLACE_TYPE)
        );
    }

    #[test]
    fn under_video_title_truncates_display_name_and_rejects_overlong_explicit_title() {
        assert_eq!(
            under_video_title("示例商品名称超过十二个字符了", None).unwrap(),
            "示例商品名称超过十二个字"
        );
        assert_eq!(
            under_video_title("很长的商品名", Some("示例框下标题")).unwrap(),
            "示例框下标题"
        );
        let error = under_video_title("商品", Some("这是一个超过十二个字符的标题"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("最多 12 个字符"));
        assert_eq!(
            truncate_chars("示例商品名称超过十二个字符", UNDER_VIDEO_TITLE_MAX_CHARS)
                .chars()
                .count(),
            12
        );
    }

    #[test]
    fn parse_main_image_url_reads_goods_detail_data() {
        let response = json!({
            "success": true,
            "data": {
                "shop_goods_id": 12345678,
                "shop_goods_name": "示例商品",
                "main_image_url": "https://example.com/cover.png"
            },
            "code": 0,
            "message": "success"
        });
        assert_eq!(
            parse_main_image_url(&response).unwrap(),
            "https://example.com/cover.png"
        );
        assert!(parse_main_image_url(&json!({"code": 0, "data": {}})).is_err());
    }

    #[test]
    fn expected_item_id_blocks_mismatched_search_result() {
        assert!(validate_expected_item_id(&up_store_item(), None).is_ok());
        assert!(validate_expected_item_id(&up_store_item(), Some("12345678")).is_ok());
        let error = validate_expected_item_id(&up_store_item(), Some("999"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("期望 999"));
        assert!(error.contains("12345678"));
    }

    #[test]
    fn attach_plan_skips_cart_when_already_selected() {
        let mut item = up_store_item();
        item["inSelectionCarState"] = json!(1);
        let plan = GoodsAttachPlan {
            cart_payload: json!({}),
            attach_payload: json!({}),
            item,
        };
        assert!(!plan.needs_add_to_cart());
    }

    #[test]
    fn failed_res_codes_are_collected_from_nested_payloads() {
        let ok = json!({"code": 0, "data": [{"resCode": 0}]});
        assert!(collect_failed_res_codes(&ok).is_empty());
        let failed = json!({"code": 0, "data": {"list": [{"resCode": 12}, {"resCode": "0"}]}});
        assert_eq!(collect_failed_res_codes(&failed), vec!["resCode=12"]);
    }

    #[test]
    fn summary_includes_index_and_jump_url() {
        let summary = summarize_goods_item(&up_store_item(), 2);
        assert_eq!(summary["index"], json!(2));
        assert_eq!(summary["itemId"], json!(12345678));
        assert!(
            summary["jumpUrl"]
                .as_str()
                .unwrap()
                .contains("mall.bilibili.com")
        );
    }
}
