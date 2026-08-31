use biliup::uploader::bilibili::{Studio, Vid};
use biliup::uploader::util::SubmitOption;
use clap::{Parser, Subcommand};

use crate::UploadLine;
use std::path::PathBuf;

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

    // #[arg(long, default_value = "sqlx=debug,tower_http=debug,info")]
    #[arg(long, default_value = "tower_http=debug,info")]
    pub rust_log: String,
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
pub enum GoodsCommands {
    /// 通过商品链接或 itemId 精确识别可挂载会员购商品
    Search {
        /// 商品链接或纯数字 itemId
        query: String,
    },
    /// 预览或执行商品挂载，默认只打印将要提交的内容
    Attach {
        /// vid为稿件 av 或 bv 号
        vid: Vid,

        /// 商品链接或纯数字 itemId
        #[arg(short, long)]
        query: String,

        /// 搜索结果下标，默认 0
        #[arg(long, default_value = "0")]
        index: usize,

        /// 带货编辑展示位，默认 12；始终同时挂视频框下
        #[arg(long, default_value = "12")]
        place_type: u32,

        /// 商品卡片前文案
        #[arg(long, default_value = "")]
        prefix_text: String,

        /// 商品卡片后文案
        #[arg(long, default_value = "")]
        postfix_text: String,

        /// 展示别名，默认使用商品原名
        #[arg(long, default_value = "")]
        another_name: String,

        /// 视频框下标题，最多 12 个字符；默认从展示名截取
        #[arg(long, default_value = "")]
        frame_title: String,

        /// 可选的商品 ID 白名单；与选中搜索结果不一致时停止
        #[arg(long)]
        expected_item_id: Option<String>,

        /// 实际写入选品车并挂载视频
        #[arg(long)]
        execute: bool,
    },
}

fn human_size(s: &str) -> Result<u64, String> {
    let ret = match s.as_bytes() {
        [init @ .., b'K'] => parse_u8(init)? * 1000.0,
        [init @ .., b'M'] => parse_u8(init)? * 1000.0 * 1000.0,
        [init @ .., b'G'] => parse_u8(init)? * 1000.0 * 1000.0 * 1000.0,
        init => parse_u8(init)?,
    };
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
                    expected_item_id: None,
                    ..
                }
            }
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
            } if expected_item_id.as_deref() == Some("12345678")
                && another_name == "示例展示名"
                && postfix_text == "示例后缀"
                && frame_title == "示例框下标题"
        ));
    }
}
