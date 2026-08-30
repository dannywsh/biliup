use super::bilibili::{BiliBili, Vid};
use crate::error::{Kind, Result};
use serde_json::{Value, json};

const MEMBERSHIP_SHOP_SOURCE_TYPE: i64 = 5;
const SEARCH_URL: &str = "https://mall.bilibili.com/mall-cbp/web/shop_goods/search";
const ADD_TO_CART_URL: &str = "https://mall.bilibili.com/mall-cbp/web/selectionCart/item/add";
const ATTACH_URL: &str = "https://mall.bilibili.com/mall-cbp/web/task/op/batch/commit";
const SEARCH_PAGE: u32 = 1;
const SEARCH_SIZE: u32 = 20;

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

/// 过滤出可售会员购商品。
///
/// 输入：搜索接口 `data.items`。返回：`sourceType=5`、可售且跳转会员购域名的商品。
pub fn filter_membership_goods(items: &[Value]) -> Vec<Value> {
    items
        .iter()
        .filter(|item| is_membership_goods(item))
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

/// 构造会员购商品挂载请求体。
///
/// 输入：商品 ID、视频 AID、展示位和卡片文案。返回：挂载接口 JSON body。
pub fn build_attach_payload(
    item_id: &str,
    aid: u64,
    place_type: u32,
    prefix_text: &str,
    postfix_text: &str,
    another_name: &str,
) -> Value {
    json!({
        "itemId": item_id,
        "videoInfos": [{"avId": aid.to_string()}],
        "cmcInfos": [{
            "cmcPlaceType": place_type,
            "prefixText": prefix_text,
            "postfixText": postfix_text,
            "anotherName": another_name,
        }]
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

fn is_membership_goods(item: &Value) -> bool {
    json_i64(item.get("sourceType")) == Some(MEMBERSHIP_SHOP_SOURCE_TYPE)
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
    /// 向会员购带货接口发送 JSON POST。
    ///
    /// 输入：`url` 为接口地址，`body` 为 JSON 请求体。
    /// 返回：`code=0` 的完整响应；失败时带上接口 `message`。
    async fn mall_json_post(&self, url: &str, body: &Value) -> Result<Value> {
        let csrf = self.get_csrf()?;
        let response = self
            .client
            .post(url)
            .header("Origin", "https://member.bilibili.com")
            .header("Referer", "https://member.bilibili.com/")
            .header("Accept", "application/json, text/plain, */*")
            .header("X-Requested-With", "XMLHttpRequest")
            .header("csrf-token", csrf)
            .header("csrf-jct", csrf)
            .json(body)
            .send()
            .await?;
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

    /// 按会员购来源搜索可售商品。
    ///
    /// 输入：`query` 为商品检索词。返回：通过会员购校验的候选列表。
    pub async fn search_membership_goods(&self, query: &str) -> Result<Vec<Value>> {
        let query = query.trim();
        if query.is_empty() {
            return Err(Kind::Custom("检索词不能为空".to_string()));
        }
        let response = self
            .mall_json_post(
                SEARCH_URL,
                &json!({
                    "cmcFirstCatNames": "",
                    "goodsName": query,
                    "query": query,
                    "page": SEARCH_PAGE,
                    "size": SEARCH_SIZE,
                    "sourceTypes": MEMBERSHIP_SHOP_SOURCE_TYPE.to_string(),
                    "sortType": 6,
                }),
            )
            .await?;
        Ok(filter_membership_goods(&search_items(&response)?))
    }

    /// 预览会员购挂载：搜索、校验商品并构造请求体，不发起写操作。
    ///
    /// 输入：检索词、稿件、结果下标、展示位、卡片文案和可选商品 ID 白名单。
    /// 返回：可供确认或随后执行的挂载计划。
    #[allow(clippy::too_many_arguments)]
    pub async fn plan_goods_attach(
        &self,
        query: &str,
        vid: &Vid,
        index: usize,
        place_type: u32,
        prefix_text: &str,
        postfix_text: &str,
        another_name: &str,
        expected_item_id: Option<&str>,
    ) -> Result<GoodsAttachPlan> {
        let candidates = self.search_membership_goods(query).await?;
        if candidates.is_empty() {
            return Err(Kind::Custom(
                "未找到可售会员购商品；请调整检索词，不要降级到非会员购来源。".to_string(),
            ));
        }
        let item = candidates.get(index).cloned().ok_or_else(|| {
            Kind::Custom(format!(
                "候选下标 {index} 超出范围，共 {} 个候选。",
                candidates.len()
            ))
        })?;
        validate_expected_item_id(&item, expected_item_id)?;
        let item_id = required_item_id(&item)?;
        let display_name = if another_name.trim().is_empty() {
            required_string(&item, "goodsName")?
        } else {
            another_name.to_string()
        };
        let aid = self.aid_from_vid(vid).await?;
        Ok(GoodsAttachPlan {
            cart_payload: build_cart_payload(&item, SEARCH_PAGE, index)?,
            attach_payload: build_attach_payload(
                &item_id,
                aid,
                place_type,
                prefix_text,
                postfix_text,
                &display_name,
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
        GoodsAttachPlan, build_attach_payload, build_cart_payload, collect_failed_res_codes,
        filter_membership_goods, summarize_goods_item, validate_expected_item_id,
    };
    use serde_json::json;

    fn membership_item() -> serde_json::Value {
        json!({
            "itemId": 12345678,
            "goodsName": "示例会员购商品",
            "sourceType": 5,
            "goodsStatus": true,
            "price": 99,
            "commissionFee": 12,
            "inSelectionCarState": 0,
            "jumpUrl": "https://mall.bilibili.com/detail.html?itemId=12345678"
        })
    }

    #[test]
    fn membership_filter_keeps_only_sellable_mall_items() {
        let taobao = json!({
            "itemId": "1",
            "sourceType": 1,
            "goodsStatus": true,
            "jumpUrl": "https://item.taobao.com/item.htm"
        });
        let unavailable = json!({
            "itemId": "2",
            "sourceType": 5,
            "goodsStatus": false,
            "jumpUrl": "https://mall.bilibili.com/detail.html?itemId=2"
        });
        let filtered = filter_membership_goods(&[membership_item(), taobao, unavailable]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["itemId"], json!(12345678));
    }

    #[test]
    fn cart_payload_copies_search_item_and_maps_commission() {
        let payload = build_cart_payload(&membership_item(), 1, 0).unwrap();
        assert_eq!(payload["operateSource"], json!(4));
        assert_eq!(payload["fromType"], json!(18));
        assert_eq!(payload["goods"][0]["income"], json!(12));
        assert_eq!(payload["goods"][0]["position"], json!("1-0"));
        assert_eq!(payload["goods"][0]["itemId"], json!(12345678));
    }

    #[test]
    fn attach_payload_uses_aid_string_and_place_type() {
        let payload =
            build_attach_payload("12345678", 123456789, 12, "", "示例后缀", "示例展示名");
        assert_eq!(payload["itemId"], json!("12345678"));
        assert_eq!(payload["videoInfos"][0]["avId"], json!("123456789"));
        assert_eq!(payload["cmcInfos"][0]["cmcPlaceType"], json!(12));
        assert_eq!(payload["cmcInfos"][0]["postfixText"], json!("示例后缀"));
        assert_eq!(payload["cmcInfos"][0]["anotherName"], json!("示例展示名"));
    }

    #[test]
    fn expected_item_id_blocks_mismatched_search_result() {
        assert!(validate_expected_item_id(&membership_item(), None).is_ok());
        assert!(validate_expected_item_id(&membership_item(), Some("12345678")).is_ok());
        let error = validate_expected_item_id(&membership_item(), Some("999"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("期望 999"));
        assert!(error.contains("12345678"));
    }

    #[test]
    fn attach_plan_skips_cart_when_already_selected() {
        let mut item = membership_item();
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
        let summary = summarize_goods_item(&membership_item(), 2);
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
