use super::bilibili::{BiliBili, Vid};
use crate::error::{Kind, Result};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const DISTINGUISH_URLS_URL: &str =
    "https://mall.bilibili.com/mall-cbp/web/cmc/goods/distinguish/urls";
const GOODS_DETAIL_URL: &str = "https://mall.bilibili.com/mall-cbp/web/shop_goods/id";
const ADD_TO_CART_URL: &str = "https://mall.bilibili.com/mall-cbp/web/selectionCart/item/add";
const ATTACH_URL: &str = "https://mall.bilibili.com/mall-cbp/web/task/op/batch/commit";
const CREATE_CMC_TASK_URL: &str =
    "https://mall.bilibili.com/mall-cbp/web/task/op/createCmcTask";
const MALL_PUBLIC_INFO_URL: &str = "https://mall.bilibili.com/mall-c-search/items/info";
const TICKET_PUBLIC_INFO_URL: &str = "https://show.bilibili.com/api/ticket/project/getV2";
const GOODS_SEARCH_PAGE: u32 = 1;
const ITEM_URL_TEMPLATE: &str = "https://mall.bilibili.com/detail.html?from=card_item&jumpLinkType=0&loadingShow=1&noTitleBar=1#goFrom=na&itemsId={item_id}&noReffer=true";
/// 视频框下商品卡展示位。
pub const UNDER_VIDEO_PLACE_TYPE: u32 = 1;
/// 带货编辑默认展示位。
pub const DEFAULT_CARD_PLACE_TYPE: u32 = 12;
/// 视频框下标题最大字符数。
pub const UNDER_VIDEO_TITLE_MAX_CHARS: usize = 12;
/// 单条评论蓝链允许挂载的最大商品数量。
pub const MAX_COMMENT_GOODS: usize = 20;
const UNDER_VIDEO_STYLE: u8 = 1;

/// 构造挂载计划所需的检索、展示位和文案参数。
#[derive(Debug, Clone, Copy)]
pub struct GoodsAttachOptions<'a> {
    /// 商品链接或纯数字 itemId。
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

/// 从商品对象提取搜索命令需要展示的字段。
///
/// 输入：完整商品 JSON。返回：含 `itemId`、名称、价格和跳转链接的精简对象。
pub fn summarize_goods_item(item: &Value, index: usize) -> Value {
    let detail = item.get("detail").cloned().unwrap_or_else(|| json!({}));
    json!({
        "index": index,
        "itemId": item.get("itemId"),
        "goodsName": item.get("goodsName"),
        "sourceType": item.get("sourceType"),
        "price": item.get("price"),
        "commissionFee": item.get("commissionFee"),
        "inSelectionCarState": item.get("inSelectionCarState"),
        "jumpUrl": item.get("jumpUrl"),
        "kind": detail.get("kind"),
        "brand": detail.get("brand"),
        "category": detail.get("category"),
        "city": detail.get("city"),
        "venue": detail.get("venue"),
        "address": detail.get("address"),
        "dates": detail.get("dates"),
        "merchant": detail.get("merchant"),
        "priceRange": detail.get("price"),
        "attrs": detail.get("attrs"),
        "images": detail.get("images"),
        "description": detail.get("description"),
        "summary": detail.get("summary"),
        "detail": detail,
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

/// 构造一条评论蓝链的多商品挂载请求体。
///
/// 输入：已完成商品识别的挂载计划、视频 AID。返回：`createCmcTask` 请求体；多个商品共用一个 `detailInfos`。
pub fn build_cmc_task_payload(plans: &[GoodsAttachPlan], aid: u64) -> Result<Value> {
    if plans.is_empty() {
        return Err(Kind::Custom("评论蓝链至少需要一个商品".to_string()));
    }
    if plans.len() > MAX_COMMENT_GOODS {
        return Err(Kind::Custom(format!(
            "一条评论蓝链最多挂载 {MAX_COMMENT_GOODS} 个商品，当前为 {} 个",
            plans.len()
        )));
    }
    let detail_infos = plans
        .iter()
        .enumerate()
        .map(|(index, plan)| {
            let item_id = required_item_id(&plan.item)?;
            let title = required_string(&plan.item, "goodsName")?;
            let cmc_info = plan
                .attach_payload
                .get("cmcInfos")
                .and_then(Value::as_array)
                .and_then(|infos| {
                    infos.iter().find(|info| {
                        info.get("cmcPlaceType").and_then(Value::as_u64)
                            == Some(DEFAULT_CARD_PLACE_TYPE as u64)
                    })
                })
                .cloned()
                .unwrap_or_else(|| json!({}));
            let another_name = cmc_info
                .get("anotherName")
                .and_then(Value::as_str)
                .filter(|name| *name != title)
                .unwrap_or("");
            let prefix_text = if index == 0 {
                cmc_info
                    .get("prefixText")
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                    .map(|text| format!("{text}\n"))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let postfix_text = if index + 1 < plans.len() { "\n" } else { "" };
            Ok(json!({
                "cmcPlaceType": DEFAULT_CARD_PLACE_TYPE,
                "title": title,
                "itemId": item_id,
                "anotherName": another_name,
                "postfixText": postfix_text,
                "prefixText": prefix_text,
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        "cmcInfos": [{
            "avId": aid.to_string(),
            "fromType": 3,
            "masTaskId": 0,
            "detailInfos": detail_infos,
        }],
        "requestFrom": 109,
    }))
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

/// 将会员购或票务 URL 转成详情接口需要的数字 ID。
/// 输入：用户传入的商品链接或 itemId。返回：`(item_id, is_ticket)`。
fn parse_public_detail_query(input: &str) -> Result<(String, bool)> {
    let input = input.trim();
    if input.is_empty() {
        return Err(Kind::Custom("商品链接或 itemId 不能为空".to_string()));
    }
    if input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok((input.to_string(), false));
    }
    let url = reqwest::Url::parse(input)
        .map_err(|_| Kind::Custom("商品链接必须是有效 URL 或纯数字 itemId".to_string()))?;
    let is_ticket = url.host_str() == Some("show.bilibili.com");
    let is_mall = url.host_str() == Some("mall.bilibili.com");
    if !is_ticket && !is_mall {
        return Err(Kind::Custom(
            "商品链接必须来自 mall.bilibili.com 或 show.bilibili.com".to_string(),
        ));
    }
    let key = if is_ticket { "id" } else { "itemsId" };
    let from_pairs = |pairs: &str| {
        url::form_urlencoded::parse(pairs.as_bytes())
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .map(|(_, value)| value.into_owned())
    };
    let item_id = from_pairs(url.query().unwrap_or(""))
        .or_else(|| from_pairs(url.fragment().unwrap_or("")))
        .filter(|value| value.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| Kind::Custom(format!("链接缺少有效的 {key} 参数")))?;
    Ok((item_id, is_ticket))
}

/// 将详情字段转换成适合展示的字符串。
/// 输入：任意 JSON 值。返回：字符串值或 JSON 紧凑表示。
fn detail_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

/// 将图片 URL 规范化为绝对 HTTPS 地址。
/// 输入：详情接口返回的图片地址。返回：可选的绝对地址。
fn absolute_image_url(url: &str) -> Option<String> {
    if url.is_empty() {
        None
    } else if url.starts_with("//") {
        Some(format!("https:{url}"))
    } else {
        Some(url.to_string())
    }
}

/// 从公开会员购详情响应提取可读商品信息。
/// 输入：`items/info` 的完整 JSON 和兜底 itemId。返回：标准化详情对象。
pub fn parse_mall_public_detail(response: &Value, fallback_item_id: &str) -> Option<Value> {
    let data = response.get("data")?.as_object()?;
    if response.get("success").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let mut attrs = BTreeMap::new();
    if let Some(list) = data.get("attrList").and_then(Value::as_array) {
        for attr in list {
            let Some(attr) = attr.as_object() else {
                continue;
            };
            let Some(name) = attr.get("attrName").and_then(Value::as_str) else {
                continue;
            };
            let value = attr.get("attrValue").map(detail_string).unwrap_or_default();
            attrs.insert(name.to_string(), value);
        }
    }
    let images = data
        .get("img")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| absolute_image_url(item.as_str()?))
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let item_id = data
        .get("itemsId")
        .map(detail_string)
        .unwrap_or_else(|| fallback_item_id.to_string());
    let price = data
        .get("price")
        .or_else(|| data.get("maxPrice"))
        .map(detail_string)
        .unwrap_or_default();
    let mut summary = vec![data.get("name").map(detail_string).unwrap_or_default()];
    if let Some(brand) = data
        .get("brandName")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        summary.push(format!("品牌{brand}"));
    }
    if !price.is_empty() {
        summary.push(format!("售价{price}元"));
    }
    summary.extend(attrs.iter().map(|(key, value)| format!("{key}{value}")));
    Some(json!({
        "kind": "mall", "itemsId": item_id, "name": data.get("name"),
        "brand": data.get("brandName").cloned().unwrap_or(Value::Null),
        "category": data.get("cateLogicNameList").cloned().unwrap_or_else(|| json!([])),
        "price": if price.is_empty() { String::new() } else { format!("{price}元") },
        "attrs": attrs, "images": images,
        "url": format!("https://mall.bilibili.com/detail.html?itemsId={item_id}"),
        "summary": summary.into_iter().filter(|value| !value.is_empty()).collect::<Vec<_>>().join("，"),
    }))
}

/// 从公开票务详情响应提取演出、场馆、日期和票价信息。
/// 输入：`getV2` 的完整 JSON 和兜底 itemId。返回：标准化详情对象。
pub fn parse_ticket_public_detail(response: &Value, fallback_item_id: &str) -> Option<Value> {
    let data = response.get("data")?.as_object()?;
    if response.get("success").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let venue = data.get("venue_info").and_then(Value::as_object);
    let city = venue
        .and_then(|value| {
            value
                .get("city_name")
                .or_else(|| value.get("province_name"))
        })
        .map(detail_string)
        .unwrap_or_default();
    let place = venue
        .and_then(|value| value.get("name"))
        .map(detail_string)
        .unwrap_or_default();
    let address = venue
        .and_then(|value| value.get("address_detail"))
        .map(detail_string)
        .unwrap_or_default();
    let dates = format_timestamp_range(data.get("start_time"), data.get("end_time"));
    let low = data.get("price_low").map(detail_string).unwrap_or_default();
    let high = data
        .get("price_high")
        .map(detail_string)
        .unwrap_or_default();
    let price = format_fen_price(&low, &high);
    let item_id = data
        .get("id")
        .map(detail_string)
        .unwrap_or_else(|| fallback_item_id.to_string());
    let images = data
        .get("cover")
        .and_then(Value::as_str)
        .and_then(absolute_image_url)
        .into_iter()
        .collect::<Vec<_>>();
    let summary = [
        data.get("name").map(detail_string).unwrap_or_default(),
        city.clone(),
        place.clone(),
        dates.clone(),
        if price.is_empty() {
            String::new()
        } else {
            format!("票价{price}")
        },
    ]
    .into_iter()
    .filter(|value| !value.is_empty())
    .collect::<Vec<_>>()
    .join("，");
    Some(json!({
        "kind": "ticket", "itemsId": item_id, "name": data.get("name"),
        "city": city, "venue": place, "address": address, "dates": dates, "price": price,
        "merchant": data.get("merchant").and_then(|value| value.get("company")).cloned().unwrap_or(Value::Null),
        "images": images, "url": format!("https://show.bilibili.com/platform/detail.html?id={item_id}"),
        "summary": summary, "description": strip_html(data.get("description").and_then(Value::as_str).unwrap_or("")),
    }))
}

/// 将分转换成人民币价格文本，支持单价或最低/最高价区间。
/// 输入：最低价和最高价字符串。返回：如 `99-299元` 的文本。
fn format_fen_price(low: &str, high: &str) -> String {
    let yuan = |value: &str| {
        value
            .parse::<i64>()
            .map(|fen| {
                if fen % 100 == 0 {
                    format!("{}元", fen / 100)
                } else {
                    format!("{}.{:02}元", fen / 100, fen.abs() % 100)
                }
            })
            .unwrap_or_else(|_| value.to_string())
    };
    if low.is_empty() {
        return yuan(high);
    }
    if high.is_empty() || low == high {
        return yuan(low);
    }
    format!("{}-{}", yuan(low).trim_end_matches('元'), yuan(high))
}

/// 将 Unix 时间戳格式化为演出日期范围。
/// 输入：开始和结束时间戳。返回：`YYYY-MM-DD` 或日期区间。
fn format_timestamp_range(start: Option<&Value>, end: Option<&Value>) -> String {
    let format_one = |value: &Value| {
        value
            .as_i64()
            .and_then(|stamp| chrono::DateTime::from_timestamp(stamp, 0))
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    };
    let start = start.map(format_one).unwrap_or_default();
    let end = end.map(format_one).unwrap_or_default();
    if start.is_empty() {
        end
    } else if end.is_empty() || start == end {
        start
    } else {
        format!("{start} ~ {end}")
    }
}

/// 去除票务简介中的 HTML 标签并限制输出长度。
/// 输入：原始 HTML 文本。返回：纯文本简介。
fn strip_html(raw: &str) -> String {
    regex::Regex::new(r"<[^>]+>")
        .map(|regex| regex.replace_all(raw, " ").to_string())
        .unwrap_or_else(|_| raw.to_string())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(800)
        .collect()
}

/// 将用户输入规范化为会员购识别接口所需的商品链接。
///
/// 输入：完整的 `mall.bilibili.com` 商品链接，或纯数字 `itemId`。
/// 返回：可直接放入 `itemUrls` 的商品链接；输入无效时返回错误。
pub fn normalize_goods_url(input: &str) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        return Err(Kind::Custom("商品链接或 itemId 不能为空".to_string()));
    }
    if input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(ITEM_URL_TEMPLATE.replace("{item_id}", input));
    }
    let url = reqwest::Url::parse(input)
        .map_err(|_| Kind::Custom("商品链接必须是有效 URL 或纯数字 itemId".to_string()))?;
    if url.scheme() != "https" || url.host_str() != Some("mall.bilibili.com") {
        return Err(Kind::Custom(
            "商品链接必须来自 https://mall.bilibili.com/".to_string(),
        ));
    }
    Ok(input.to_string())
}

/// 提取并规范化商品链接识别接口的成功结果。
///
/// 输入：`distinguish/urls` 的完整 JSON 响应。
/// 返回：可供选品车和挂载接口复用的商品列表；接口结构或识别失败时返回错误。
pub fn distinguish_goods_items(response: &Value) -> Result<Vec<Value>> {
    let items = response
        .pointer("/data/successList")
        .and_then(Value::as_array)
        .ok_or_else(|| Kind::Custom("商品识别接口缺少 data.successList".to_string()))?;
    let failures = response
        .pointer("/data/failList")
        .and_then(Value::as_array)
        .map(|list| list.len())
        .unwrap_or(0);
    let mut result = Vec::with_capacity(items.len());
    for item in items {
        if json_i64(item.get("distinguishState")).is_some_and(|state| state != 0) {
            continue;
        }
        let mut normalized = item
            .get("goodsDto")
            .cloned()
            .filter(Value::is_object)
            .ok_or_else(|| Kind::Custom("商品识别结果缺少 goodsDto".to_string()))?;
        let object = normalized
            .as_object_mut()
            .expect("goodsDto has been verified as object");
        for key in [
            "itemId",
            "outerId",
            "sourceType",
            "mainImgUrl",
            "goodsName",
            "price",
            "commissionFee",
            "commissionRate",
        ] {
            if let Some(value) = item.get(key) {
                object.insert(key.to_string(), value.clone());
            }
        }
        object.insert(
            "inSelectionCarState".to_string(),
            item.get("inSelectionCarState").cloned().unwrap_or(json!(0)),
        );
        result.push(normalized);
    }
    if result.is_empty() && failures > 0 {
        return Err(Kind::Custom(
            "商品链接识别失败，请确认链接或 itemId 有效且可挂载".to_string(),
        ));
    }
    Ok(result)
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
        let payload = Self::json_from_response(response).await?;
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
        let csrf = self.get_csrf()?;
        let request =
            self.with_mall_headers(self.client.post(url).query(&[("csrf", csrf)]).json(body))?;
        self.send_mall_request(request).await
    }

    /// 向会员购带货接口发送 GET。
    ///
    /// 输入：`url` 为接口地址，`query` 为查询参数。返回：`code=0` 的完整响应。
    async fn mall_json_get(&self, url: &str, query: &[(&str, &str)]) -> Result<Value> {
        let request = self.with_mall_headers(self.client.get(url).query(query))?;
        self.send_mall_request(request).await
    }

    /// 请求无需登录即可读取的会员购/票务公开详情接口。
    /// 输入：详情接口地址、查询参数和来源页。返回：完整 JSON 响应。
    async fn public_json_get(
        &self,
        url: &str,
        query: &[(&str, &str)],
        referer: &str,
    ) -> Result<Value> {
        let response = self.client.get(url)
            .query(query)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/131.0.0.0 Safari/537.36")
            .header("Referer", referer)
            .header("Accept", "application/json,text/plain,*/*")
            .send().await?;
        Self::json_from_response(response).await
    }

    /// 将公开详情附加到识别结果，保留原商品字段以兼容挂载流程。
    /// 输入：识别结果和详情查询 ID。返回：包含 `detail` 的商品对象。
    async fn enrich_goods_item(&self, mut item: Value, item_id: &str) -> Value {
        let detail = self
            .public_json_get(
                MALL_PUBLIC_INFO_URL,
                &[("itemsId", item_id)],
                &format!("https://mall.bilibili.com/detail.html?itemsId={item_id}"),
            )
            .await
            .ok()
            .and_then(|payload| parse_mall_public_detail(&payload, item_id));
        if let Some(detail) = detail {
            if let Some(object) = item.as_object_mut() {
                object.insert("detail".to_string(), detail);
            }
        }
        item
    }

    /// 直接构造票务商品搜索结果，并附加公开票务详情。
    /// 输入：票务 itemId。返回：可供 `goods search` 展示的商品对象。
    async fn search_ticket_goods(&self, item_id: &str) -> Result<Value> {
        let payload = self
            .public_json_get(
                TICKET_PUBLIC_INFO_URL,
                &[("id", item_id)],
                &format!("https://show.bilibili.com/platform/detail.html?id={item_id}"),
            )
            .await?;
        let detail = parse_ticket_public_detail(&payload, item_id)
            .ok_or_else(|| Kind::Custom("票务详情接口未返回有效项目".to_string()))?;
        let name = detail.get("name").cloned().unwrap_or(Value::Null);
        let price = detail.get("price").cloned().unwrap_or(Value::Null);
        Ok(json!({
            "itemId": detail.get("itemsId"), "goodsName": name, "price": price,
            "jumpUrl": detail.get("url"), "sourceType": "ticket", "detail": detail,
        }))
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

    /// 按商品链接精确识别可挂载商品。
    ///
    /// 输入：完整商品链接或纯数字 `itemId`。返回：链接识别出的商品列表，不进行标题模糊匹配。
    pub async fn search_goods(&self, query: &str) -> Result<Vec<Value>> {
        let numeric_query = query.trim().bytes().all(|byte| byte.is_ascii_digit());
        let (item_id, is_ticket) = parse_public_detail_query(query)?;
        if is_ticket {
            return Ok(vec![self.search_ticket_goods(&item_id).await?]);
        }
        let item_url = normalize_goods_url(query)?;
        if let Ok(response) = self
            .mall_json_post(DISTINGUISH_URLS_URL, &json!({"itemUrls": item_url}))
            .await
        {
            if let Ok(items) = distinguish_goods_items(&response) {
                let mut enriched = Vec::with_capacity(items.len());
                for item in items {
                    enriched.push(self.enrich_goods_item(item, &item_id).await);
                }
                return Ok(enriched);
            }
        }
        let mall_payload = self
            .public_json_get(
                MALL_PUBLIC_INFO_URL,
                &[("itemsId", &item_id)],
                &format!("https://mall.bilibili.com/detail.html?itemsId={item_id}"),
            )
            .await;
        if let Ok(payload) = mall_payload {
            if let Some(detail) = parse_mall_public_detail(&payload, &item_id) {
                return Ok(vec![json!({
                    "itemId": detail.get("itemsId"), "goodsName": detail.get("name"),
                    "price": detail.get("price"), "jumpUrl": detail.get("url"),
                    "detail": detail,
                })]);
            }
        }
        if numeric_query {
            return Ok(vec![self.search_ticket_goods(&item_id).await?]);
        }
        Err(Kind::Custom("会员购详情接口未返回有效商品".to_string()))
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
                "未识别到可挂载商品；请确认商品链接或 itemId。".to_string(),
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
            cart_payload: build_cart_payload(&item, GOODS_SEARCH_PAGE, options.index)?,
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

    /// 执行多个商品合并为一条评论蓝链的挂载。
    ///
    /// 输入：已完成预检的商品计划。返回：选品车响应数组与单条评论挂载响应。
    pub async fn execute_cmc_task(
        &self,
        plans: &[GoodsAttachPlan],
    ) -> Result<(Value, Value)> {
        if plans.len() > MAX_COMMENT_GOODS {
            return Err(Kind::Custom(format!(
                "一条评论蓝链最多挂载 {MAX_COMMENT_GOODS} 个商品，当前为 {} 个",
                plans.len()
            )));
        }
        let mut cart_results = Vec::with_capacity(plans.len());
        for plan in plans {
            let cart_result = if plan.needs_add_to_cart() {
                self.mall_json_post(ADD_TO_CART_URL, &plan.cart_payload)
                    .await?
            } else {
                json!("already_in_selection_cart")
            };
            cart_results.push(cart_result);
        }
        let aid = plans
            .first()
            .and_then(|plan| plan.attach_payload.pointer("/videoInfos/0/avId"))
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| Kind::Custom("商品挂载计划缺少有效视频 AID".to_string()))?;
        let payload = build_cmc_task_payload(plans, aid)?;
        let result = self
            .mall_json_post(CREATE_CMC_TASK_URL, &payload)
            .await?;
        if result.get("code").and_then(Value::as_i64) != Some(0) {
            return Err(Kind::Custom(format!(
                "评论蓝链挂载接口返回失败：{result}"
            )));
        }
        Ok((Value::Array(cart_results), result))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_CARD_PLACE_TYPE, GoodsAttachPlan, UNDER_VIDEO_PLACE_TYPE,
        UNDER_VIDEO_TITLE_MAX_CHARS, build_attach_payload, build_cart_payload,
        build_cmc_task_payload,
        collect_failed_res_codes, distinguish_goods_items, normalize_goods_url,
        parse_main_image_url, parse_mall_public_detail, parse_ticket_public_detail,
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

    #[test]
    fn numeric_item_id_is_converted_to_membership_shop_url() {
        let url = normalize_goods_url("12345678").unwrap();
        assert!(url.contains("itemsId=12345678"));
        assert!(normalize_goods_url("示例商品").is_err());
        assert!(normalize_goods_url("https://example.com/item/12345678").is_err());
    }

    #[test]
    fn public_mall_detail_contains_product_information() {
        let detail = parse_mall_public_detail(
            &json!({
                "success": true,
                "data": {
                    "itemsId": 12345678,
                    "name": "示例周边",
                    "brandName": "示例品牌",
                    "cateLogicNameList": ["手办"],
                    "price": 199,
                    "attrList": [{"attrName": "材质", "attrValue": ["PVC"]}],
                    "img": ["//example.com/a.png"]
                }
            }),
            "12345678",
        )
        .unwrap();
        assert_eq!(detail["kind"], json!("mall"));
        assert_eq!(detail["brand"], json!("示例品牌"));
        assert_eq!(detail["attrs"]["材质"], json!("[\"PVC\"]"));
        assert_eq!(detail["images"][0], json!("https://example.com/a.png"));
    }

    #[test]
    fn public_ticket_detail_contains_event_information() {
        let detail = parse_ticket_public_detail(&json!({
            "success": true,
            "data": {
                "id": 1004629,
                "name": "示例演出",
                "start_time": 1760000000,
                "end_time": 1760086400,
                "price_low": 9900,
                "price_high": 29900,
                "cover": "//example.com/cover.jpg",
                "venue_info": {"city_name": "上海", "name": "示例剧院", "address_detail": "示例路 1 号"},
                "merchant": {"company": "示例主办方"}
            }
        }), "1004629").unwrap();
        assert_eq!(detail["kind"], json!("ticket"));
        assert_eq!(detail["city"], json!("上海"));
        assert_eq!(detail["venue"], json!("示例剧院"));
        assert_eq!(detail["price"], json!("99-299元"));
    }

    #[test]
    fn distinguish_response_is_normalized_for_cart_and_attach() {
        let response = json!({
            "code": 0,
            "data": {
                "failList": [],
                "successList": [{
                    "itemId": "12345678",
                    "sourceType": 5,
                    "goodsName": "示例商品",
                    "mainImgUrl": "https://i0.hdslb.com/example.png",
                    "distinguishState": 0,
                    "goodsDto": {
                        "itemId": 12345678,
                        "goodsStatus": 1,
                        "jumpUrl": "https://mall.bilibili.com/detail.html?itemsId=12345678",
                        "commissionFee": 7.68
                    }
                }]
            }
        });
        let items = distinguish_goods_items(&response).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["itemId"], json!("12345678"));
        assert_eq!(items[0]["inSelectionCarState"], json!(0));
        assert_eq!(items[0]["goodsName"], json!("示例商品"));
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
    fn cmc_task_payload_puts_multiple_goods_in_one_comment() {
        let first = GoodsAttachPlan {
            item: json!({"itemId": "13667449", "goodsName": "角川 英雄传说 轨迹系列 版画"}),
            cart_payload: json!({}),
            attach_payload: json!({"cmcInfos": [{"cmcPlaceType": 12, "anotherName": "角川 英雄传说 轨迹系列 版画", "prefixText": "大家都想要的同款，都在这里啦！", "postfixText": ""}]}),
        };
        let second = GoodsAttachPlan {
            item: json!({"itemId": "13667450", "goodsName": "角川 英雄传说 轨迹系列 毛绒玩偶挂件"}),
            cart_payload: json!({}),
            attach_payload: json!({"cmcInfos": [{"cmcPlaceType": 12, "anotherName": "角川 英雄传说 轨迹系列 毛绒玩偶挂件", "prefixText": "", "postfixText": ""}]}),
        };
        let payload = build_cmc_task_payload(&[first, second], 117251432844284).unwrap();
        assert_eq!(payload["requestFrom"], json!(109));
        assert_eq!(payload["cmcInfos"][0]["detailInfos"].as_array().unwrap().len(), 2);
        assert_eq!(payload["cmcInfos"][0]["detailInfos"][0]["itemId"], json!("13667449"));
        assert_eq!(payload["cmcInfos"][0]["detailInfos"][1]["itemId"], json!("13667450"));
        assert_eq!(payload["cmcInfos"][0]["detailInfos"][0]["prefixText"], json!("大家都想要的同款，都在这里啦！\n"));
        assert_eq!(payload["cmcInfos"][0]["detailInfos"][0]["postfixText"], json!("\n"));
        assert_eq!(payload["cmcInfos"][0]["detailInfos"][1]["prefixText"], json!(""));
        assert_eq!(payload["cmcInfos"][0]["detailInfos"][1]["postfixText"], json!(""));
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
