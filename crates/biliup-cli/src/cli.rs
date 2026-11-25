use biliup::uploader::bilibili::{Studio, Vid};
use biliup::uploader::util::SubmitOption;
use clap::{Parser, Subcommand};

use crate::UploadLine;
use crate::server::fleet::RelayListen;
use std::path::PathBuf;
use url::Url;

/// 扩展路径中的 ~ 为用户主目录
pub fn expand_path(path: PathBuf) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        let expanded = shellexpand::tilde(path_str);
        return PathBuf::from(expanded.as_ref());
    }
    path
}

#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    // /// Turn debugging information on
    // #[clap(short, long, parse(from_occurrences))]
    // debug: usize,
    #[clap(subcommand)]
    pub command: Commands,

    /// 配置代理
    #[arg(short, long, default_value = None)]
    pub proxy: Option<String>,

    /// 登录信息文件
    #[arg(short, long, default_value = "cookies.json")]
    pub user_cookie: PathBuf,

    /// 日志过滤规则，如 debug；不指定时读取环境变量 RUST_LOG，都没有则为 tower_http=debug,info
    #[arg(long)]
    pub rust_log: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 登录B站并保存登录信息
    Login,
    /// 手动验证并刷新登录信息
    Renew,
    /// 上传视频
    Upload {
        /// 提交接口
        #[arg(long)]
        submit: Option<SubmitOption>,

        // Optional name to operate on
        // name: Option<String>,
        /// 需要上传的视频路径,若指定配置文件投稿不需要此参数
        #[arg()]
        video_path: Vec<PathBuf>,

        /// Sets a custom config file
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// 选择上传线路
        #[arg(short, long, value_enum)]
        line: Option<UploadLine>,

        /// 单视频文件最大并发数
        #[arg(long, default_value = "3")]
        limit: usize,

        #[command(flatten)]
        studio: Studio,
        // #[arg(required = false, last = true, default_value = "client")]
        // submit: Option<String>,
    },
    /// 是否要对某稿件追加视频
    Append {
        /// 提交接口
        #[arg(long)]
        submit: Option<SubmitOption>,

        // Optional name to operate on
        // name: Option<String>,
        /// vid为稿件 av 或 bv 号
        #[arg(short, long)]
        vid: Vid,
        /// 需要上传的视频路径,若指定配置文件投稿不需要此参数
        #[arg()]
        video_path: Vec<PathBuf>,

        /// 选择上传线路
        #[arg(short, long, value_enum)]
        line: Option<UploadLine>,

        /// 单视频文件最大并发数
        #[arg(long, default_value = "3")]
        limit: usize,

        #[command(flatten)]
        studio: Studio,
    },
    /// 打印视频详情
    Show {
        /// vid为稿件 av 或 bv 号
        // #[clap()]
        vid: Vid,
    },
    /// 查看视频评论
    Comments {
        /// vid为稿件 av 或 bv 号
        vid: Vid,

        /// 排序方式，0为按时间，2为按热度
        #[arg(long, default_value = "0")]
        sort: u8,

        /// 页码
        #[arg(long, default_value = "1")]
        pn: u32,

        /// 每页条数
        #[arg(long, default_value = "20")]
        ps: u32,
    },
    /// 回复视频评论，默认只打印将要回复的内容
    Reply {
        /// vid为稿件 av 或 bv 号
        vid: Vid,

        /// 评论 rpid
        rpid: u64,

        /// 回复内容
        message: String,

        /// 实际发送回复
        #[arg(long)]
        execute: bool,
    },
    /// 置顶或取消置顶视频评论，默认只打印将要执行的操作
    TopReply {
        /// vid为稿件 av 或 bv 号
        vid: Vid,

        /// 评论 rpid
        rpid: u64,

        /// 取消置顶；默认置顶
        #[arg(long)]
        unpin: bool,

        /// 实际发送置顶请求
        #[arg(long)]
        execute: bool,
    },
    /// 管理新版合集（SEASON）
    Season {
        #[command(subcommand)]
        command: SeasonCommands,
    },
    /// 搜索商品，或挂载到已发布视频
    Goods {
        #[command(subcommand)]
        command: GoodsCommands,
    },
    /// 输出flv元数据
    DumpFlv {
        #[arg()]
        file_name: PathBuf,
    },
    /// 下载视频
    Download {
        url: String,

        /// Output filename template. e.p. "./video/%Y-%m-%dT%H_%M_%S{title}"
        #[arg(short, long, default_value = "{title}")]
        output: String,

        /// 按照大小分割视频
        #[arg(long, value_parser = human_size)]
        split_size: Option<u64>,

        /// 按照时间分割视频
        #[arg(long)]
        split_time: Option<humantime::Duration>,
    },
    /// 启动web服务，默认端口19159
    Server {
        /// Specify bind address
        #[arg(short, long, default_value = "127.0.0.1")]
        bind: String,

        /// Port to use
        #[arg(short, long, default_value = "19159")]
        port: u16,

        /// 开启登录密码认证
        #[arg(long, default_value = "false")]
        auth: bool,

        /// 为会话 Cookie 附加 Secure 属性。仅当通过 HTTPS 反向代理访问 Web UI 时开启；
        /// 直接通过 HTTP 远程访问时开启会导致浏览器丢弃登录态
        #[arg(long, default_value = "false")]
        secure_session_cookie: bool,

        /// 使用 biliup 1.0.7 风格配置文件启动录制
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// 显式以 Fleet 控制面运行（默认自动开启；已有节点身份时作为节点运行）
        #[arg(long)]
        controller: bool,

        /// 控制面内嵌 relay 的 TCP 监听地址（默认 0.0.0.0:19160），节点必须能连到它；
        /// off 表示不起内嵌 relay，此时必须用 --relay-url 指定外部 relay。作为节点运行时忽略
        #[arg(long, value_name = "ADDR|off")]
        relay_listen: Option<RelayListen>,

        /// 写进加入票据的 relay 地址，可重复；不给时自动列出本机网卡地址。
        /// 用于域名、端口转发、反向代理或外部 relay。作为节点运行时忽略
        #[arg(long, value_name = "URL")]
        relay_url: Vec<Url>,
    },
    /// 管理 Web 界面的登录用户（在 biliup 服务的工作目录下执行，直接读写 data/data.sqlite3）
    User {
        #[command(subcommand)]
        action: UserAction,
    },
    /// 把这台机器加入 Fleet 控制面或退出（在 biliup 服务的工作目录下执行，读写 data/node.json）
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },
    /// 列出所有已上传的视频
    List {
        /// 只包含进行中的视频
        #[arg(long)]
        is_pubing: bool,

        /// 只包含已通过的视频
        #[arg(long)]
        pubed: bool,

        /// 只包含未通过的视频
        #[arg(long)]
        not_pubed: bool,

        /// 从第几页开始获取
        #[arg(short, long, default_value = "1")]
        from_page: u32,

        /// 最大获取页数
        #[arg(short, long)]
        max_pages: Option<u32>,
    },
}

#[derive(Subcommand)]
pub enum SeasonCommands {
    /// 列出当前账号的合集
    List {
        /// 从第几页开始获取
        #[arg(long, default_value = "1")]
        pn: u32,

        /// 每页合集数量
        #[arg(long, default_value = "30")]
        ps: u32,
    },
    /// 创建新版合集，默认只预览请求
    Create {
        /// 合集标题
        #[arg(long)]
        title: String,

        /// 合集简介
        #[arg(long, default_value = "")]
        desc: String,

        /// 合集封面 URL
        #[arg(long)]
        cover: String,

        /// 合集价格，免费合集保持为 0
        #[arg(long, default_value = "0")]
        season_price: u32,

        /// 实际提交创建操作
        #[arg(long)]
        execute: bool,
    },
    /// 查看合集分区内的视频
    Episodes {
        /// 合集分区 ID
        #[arg(long)]
        section_id: u64,

        /// 可选排序方式，例如 desc
        #[arg(long)]
        sort: Option<String>,
    },
    /// 添加视频到合集，默认只预览请求
    Add {
        /// 合集分区 ID
        #[arg(long)]
        section_id: u64,

        /// 视频 AV/BV 号，可重复传入
        #[arg(long = "vid", required = true, num_args = 1..)]
        vids: Vec<Vid>,

        /// 实际提交添加操作
        #[arg(long)]
        execute: bool,
    },
    /// 修改合集内视频标题，默认只预览请求
    Edit {
        /// 合集分区 ID
        #[arg(long)]
        section_id: u64,

        /// 合集内部视频 episode ID，不是 aid 或 BV 号
        #[arg(long)]
        episode_id: u64,

        /// 修改后的合集内标题
        #[arg(long)]
        title: String,

        /// 实际提交标题修改操作
        #[arg(long)]
        execute: bool,
    },
    /// 从合集移除视频，默认只预览请求
    Remove {
        /// 合集内部视频 episode ID，不是 aid 或 BV 号
        #[arg(long)]
        episode_id: u64,

        /// 实际提交移除操作
        #[arg(long)]
        execute: bool,
    },
    /// 重新排序合集分区，默认只预览请求
    Sort {
        /// 合集 ID
        #[arg(long)]
        season_id: u64,

        /// 合集分区 ID
        #[arg(long)]
        section_id: u64,

        /// 合集分区标题
        #[arg(long, default_value = "正片")]
        section_title: String,

        /// 按目标顺序传入全部 episode ID，可重复传入
        #[arg(long = "episode-id", required = true, num_args = 1..)]
        episode_ids: Vec<u64>,

        /// 实际提交排序操作
        #[arg(long)]
        execute: bool,
    },
}

#[derive(Subcommand)]
pub enum UserAction {
    /// 列出所有 Web 用户
    List,
    /// 重置某个 Web 用户的密码并让其所有会话失效；新密码从终端提示输入，或从标准输入读一行
    ResetPassword {
        /// 用户名（大小写不敏感）
        username: String,
    },
}

#[derive(Subcommand)]
pub enum GoodsCommands {
    /// 申请会员购测评团，默认预览请求
    ApplyReview {
        /// 正整数会员购商品 itemId
        item_id: String,
        /// 以 BV1 开头的 12 位视频号
        bv_id: String,
        /// 实际提交申请
        #[arg(long)]
        execute: bool,
    },
    /// 按商品链接或 itemId 精确查询商品；会员购结果包含活动、优惠券和 SKU 促销信息
    Search {
        /// 商品链接或纯数字 itemId；会员购商品可直接输入 itemId 查询促销
        query: String,
    },
    /// 预览或加入商品到选品车，默认只打印将要提交的内容
    Cart {
        /// 商品链接或纯数字 itemId；可重复传入或一次传入多个值
        #[arg(short, long, num_args = 1..)]
        query: Vec<String>,

        /// 搜索结果下标，默认 0
        #[arg(long, default_value = "0")]
        index: usize,

        /// 实际加入选品车
        #[arg(long)]
        execute: bool,
    },
    /// 预览或执行商品挂载，默认只打印将要提交的内容
    Attach {
        /// vid为稿件 av 或 bv 号
        vid: Vid,

        /// 商品链接或纯数字 itemId；可重复传入或一次传入多个值
        #[arg(short, long, num_args = 1..)]
        query: Vec<String>,

        /// 搜索结果下标，默认 0
        #[arg(long, default_value = "0")]
        index: usize,

        /// 单商品评论展示位，默认 12；多商品评论蓝链必须使用默认值 12
        #[arg(long, default_value = "12")]
        place_type: u32,

        /// 商品前文案；多商品时仅放在第一项前
        #[arg(long, default_value = "")]
        prefix_text: String,

        /// 商品后文案；多商品时仅放在最后一项后
        #[arg(long, default_value = "")]
        postfix_text: String,

        /// 展示别名，默认使用商品原名
        #[arg(long, default_value = "")]
        another_name: String,

        /// 单商品视频框下标题，最多 12 个字符；默认从展示名截取，多商品不使用
        #[arg(long, default_value = "")]
        frame_title: String,

        /// 可选的商品 ID 白名单；传一个值时应用于全部商品，多个值按商品顺序对应
        #[arg(long, num_args = 1..)]
        expected_item_id: Vec<String>,

        /// 实际写入选品车并挂载视频
        #[arg(long)]
        execute: bool,
    },
}

#[derive(Subcommand)]
pub enum NodeAction {
    /// 用控制面「添加节点」给出的票据加入；成功后重启 biliup server 生效
    Join {
        /// 以 bfleet 开头的加入票据
        ticket: String,
        /// 允许控制面下发的房间使用钩子（各 *processor 与 ffmpeg 参数，等于允许在本机执行命令）
        #[arg(long)]
        allow_hooks: bool,
    },
    /// 通知控制面移除本节点，并删除本地凭据 data/node.json
    Leave,
    /// 查看本机的节点凭据（不连控制面）
    Status,
}

fn human_size(s: &str) -> Result<u64, String> {
    let ret = match s.as_bytes() {
        [init @ .., b'K'] => parse_u8(init)? * 1000.0,
        [init @ .., b'M'] => parse_u8(init)? * 1000.0 * 1000.0,
        [init @ .., b'G'] => parse_u8(init)? * 1000.0 * 1000.0 * 1000.0,
        init => parse_u8(init)?,
    };
    if !ret.is_finite() || ret < 1.0 || ret >= u64::MAX as f64 {
        return Err(format!(
            "{s} must be a positive size below {} bytes",
            u64::MAX
        ));
    }
    Ok(ret as u64)
}

fn parse_u8(string: &[u8]) -> Result<f64, String> {
    let string = String::from_utf8_lossy(string);
    string
        .parse()
        .map_err(|e| format!("{string} is not ascii digit. {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::{Cli, Commands};
    use clap::Parser;
    use std::path::Path;

    #[test]
    fn download_rejects_sizes_that_would_silently_become_zero_or_overflow() {
        for size in [
            "NaN",
            "inf",
            "-1",
            "0",
            "0.5",
            "1e100",
            "18446744073709551616",
        ] {
            assert!(
                Cli::try_parse_from([
                    "biliup",
                    "download",
                    "https://example.invalid/live.flv",
                    "--split-size",
                    size,
                ])
                .is_err(),
                "accepted size {size}"
            );
        }
        assert_eq!(super::human_size("1.5M").unwrap(), 1_500_000);
    }

    #[test]
    fn server_defaults_to_loopback_and_default_cookie_file() {
        let cli = Cli::try_parse_from(["biliup", "server"]).unwrap();

        assert_eq!(cli.user_cookie, Path::new("cookies.json"));
        assert!(matches!(
            cli.command,
            Commands::Server {
                ref bind,
                auth: false,
                secure_session_cookie: false,
                ..
            } if bind == "127.0.0.1"
        ));
    }

    #[test]
    fn fleet_server_flags_parse() {
        use crate::server::fleet::RelayListen;
        let cli = Cli::try_parse_from(["biliup", "server"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Server {
                controller: false,
                relay_listen: None,
                ref relay_url,
                ..
            } if relay_url.is_empty()
        ));
        let cli = Cli::try_parse_from([
            "biliup",
            "server",
            "--controller",
            "--relay-listen",
            "off",
            "--relay-url",
            "http://relay.example.com:19160",
            "--relay-url",
            "http://10.0.0.2:19160",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Commands::Server {
                controller: true,
                relay_listen: Some(RelayListen::Off),
                ref relay_url,
                ..
            } if relay_url.len() == 2
        ));
        assert!(Cli::try_parse_from(["biliup", "server", "--relay-listen", "nope"]).is_err());
    }

    #[test]
    fn node_subcommands_parse() {
        let cli =
            Cli::try_parse_from(["biliup", "node", "join", "bfleetabc", "--allow-hooks"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Node {
                action: super::NodeAction::Join { ref ticket, allow_hooks: true }
            } if ticket == "bfleetabc"
        ));
        for (name, leave) in [("leave", true), ("status", false)] {
            let cli = Cli::try_parse_from(["biliup", "node", name]).unwrap();
            let Commands::Node { action } = cli.command else {
                panic!("not a node command")
            };
            assert_eq!(matches!(action, super::NodeAction::Leave), leave);
        }
    }

    #[test]
    fn user_subcommands_parse() {
        let cli = Cli::try_parse_from(["biliup", "user", "reset-password", "biliup"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::User {
                action: super::UserAction::ResetPassword { ref username }
            } if username == "biliup"
        ));
        let cli = Cli::try_parse_from(["biliup", "user", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::User {
                action: super::UserAction::List
            }
        ));
    }

    #[test]
    fn server_preserves_an_explicit_cookie_file() {
        let cli = Cli::try_parse_from([
            "biliup",
            "--user-cookie",
            "/tmp/private-account.json",
            "server",
        ])
        .unwrap();

        assert_eq!(cli.user_cookie, Path::new("/tmp/private-account.json"));
    }

    #[test]
    fn top_reply_defaults_to_dry_run_pin() {
        let cli = Cli::try_parse_from(["biliup", "top-reply", "BV1test", "1"]).unwrap();

        assert!(matches!(
            cli.command,
            Commands::TopReply {
                rpid: 1,
                unpin: false,
                execute: false,
                ..
            }
        ));
    }

    #[test]
    fn top_reply_accepts_execute_and_unpin() {
        let cli = Cli::try_parse_from([
            "biliup",
            "top-reply",
            "BV1test",
            "1",
            "--unpin",
            "--execute",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Commands::TopReply {
                rpid: 1,
                unpin: true,
                execute: true,
                ..
            }
        ));
    }

    #[test]
    fn season_add_accepts_repeated_vids_and_defaults_to_dry_run() {
        let cli = Cli::try_parse_from([
            "biliup",
            "season",
            "add",
            "--section-id",
            "42",
            "--vid",
            "BV1test",
            "--vid",
            "av123",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Commands::Season {
                command: super::SeasonCommands::Add {
                    section_id: 42,
                    ref vids,
                    execute: false,
                }
            } if vids == &vec![
                biliup::uploader::bilibili::Vid::Bvid("BV1test".into()),
                biliup::uploader::bilibili::Vid::Aid(123),
            ]
        ));
    }

    #[test]
    fn season_create_requires_title_and_cover_and_defaults_to_dry_run() {
        let cli = Cli::try_parse_from([
            "biliup",
            "season",
            "create",
            "--title",
            "示例合集",
            "--cover",
            "https://example.com/cover.jpg",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Commands::Season {
                command:
                    super::SeasonCommands::Create {
                        ref title,
                        ref desc,
                        ref cover,
                        season_price: 0,
                        execute: false,
                    }
            } if title == "示例合集"
                && desc.is_empty()
                && cover == "https://example.com/cover.jpg"
        ));
    }

    #[test]
    fn season_sort_accepts_all_episode_ids_and_execute() {
        let cli = Cli::try_parse_from([
            "biliup",
            "season",
            "sort",
            "--season-id",
            "7",
            "--section-id",
            "8",
            "--episode-id",
            "11",
            "--episode-id",
            "12",
            "--execute",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Commands::Season {
                command: super::SeasonCommands::Sort {
                    season_id: 7,
                    section_id: 8,
                    ref episode_ids,
                    execute: true,
                    ..
                }
            } if episode_ids == &vec![11, 12]
        ));
    }

    #[test]
    fn season_edit_requires_section_episode_title_and_defaults_to_dry_run() {
        let cli = Cli::try_parse_from([
            "biliup",
            "season",
            "edit",
            "--section-id",
            "8",
            "--episode-id",
            "11",
            "--title",
            "示例合集标题",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Commands::Season {
                command: super::SeasonCommands::Edit {
                    section_id: 8,
                    episode_id: 11,
                    ref title,
                    execute: false,
                }
            } if title == "示例合集标题"
        ));
    }

    #[test]
    fn goods_search_parses_query() {
        let cli = Cli::try_parse_from(["biliup", "goods", "search", "示例商品"]).unwrap();

        assert!(matches!(
            cli.command,
            Commands::Goods {
                command: super::GoodsCommands::Search { ref query },
            } if query == "示例商品"
        ));
    }

    #[test]
    fn goods_attach_defaults_to_dry_run() {
        let cli = Cli::try_parse_from([
            "biliup",
            "goods",
            "attach",
            "BV1test",
            "--query",
            "示例商品",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Commands::Goods {
                command: super::GoodsCommands::Attach {
                    execute: false,
                    index: 0,
                    place_type: 12,
                    expected_item_id: ref expected_item_ids,
                    ..
                }
            } if expected_item_ids.is_empty()
        ));
    }

    #[test]
    fn goods_attach_accepts_execute_and_item_guard() {
        let cli = Cli::try_parse_from([
            "biliup",
            "goods",
            "attach",
            "BV1test",
            "--query",
            "示例商品",
            "--expected-item-id",
            "12345678",
            "--another-name",
            "示例展示名",
            "--postfix-text",
            "示例后缀",
            "--frame-title",
            "示例框下标题",
            "--execute",
        ])
        .unwrap();

        assert!(matches!(
            cli.command,
            Commands::Goods {
                command: super::GoodsCommands::Attach {
                    execute: true,
                    ref expected_item_id,
                    ref another_name,
                    ref postfix_text,
                    ref frame_title,
                    ..
                }
            } if expected_item_id == &["12345678".to_string()]
                && another_name == "示例展示名"
                && postfix_text == "示例后缀"
                && frame_title == "示例框下标题"
        ));
    }
}

#[cfg(test)]
mod review_command_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn review_application_requires_explicit_execute() {
        for execute in [false, true] {
            let mut args = vec!["biliup", "goods", "apply-review", "1", "BV1TEST00000"];
            if execute {
                args.push("--execute");
            }
            let cli = Cli::try_parse_from(args).unwrap();
            assert!(matches!(cli.command, Commands::Goods {
                command: GoodsCommands::ApplyReview { execute: actual, .. }
            } if actual == execute));
        }
        assert!(Cli::try_parse_from(["biliup", "goods", "apply-review", "1"]).is_err());
    }
}
