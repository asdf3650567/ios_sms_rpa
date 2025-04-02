use axum::{extract::Query, http::StatusCode, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fs,
    sync::{Arc, Mutex},
};
use axum::serve;
use log::{info, debug};

#[derive(Debug, Deserialize)]
struct Config {
    port: u16,
    default_fetch_count: usize,
    test_number: String,
}

#[derive(Debug, Serialize)]
struct ResponseData {
    numbers: String,
    message: String,
    count: usize,
}

struct AppState {
    numbers: VecDeque<String>,
    message: String,
    start_index: usize,
    default_fetch_count: usize,
    test_number: String,
}

#[tokio::main]
async fn main() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // 加载配置文件
    let config = load_config("config.toml");
    info!("加载配置文件 => 单次取号码 {} + 1 个, 测试号：{}", config.default_fetch_count, config.test_number);

    // 加载数据
    let state = Arc::new(Mutex::new(load_state(&config)));

    // 设置路由
    let app = Router::new().route("/fetch", get(fetch_handler)).with_state(state);

    // 启动服务
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("服务器启动成功 => http://{}", addr);

    serve(listener, app.into_make_service()).await.unwrap();
}

// 处理 /fetch 请求
async fn fetch_handler(
    Query(params): Query<std::collections::HashMap<String, String>>,
    state: axum::extract::State<Arc<Mutex<AppState>>>,
) -> Result<Json<ResponseData>, StatusCode> {
    let mut state = state.lock().unwrap();
    let total_items = state.numbers.len();

    // 获取 n，如果没有提供则使用配置中的默认值
    let n = params
        .get("n")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(state.default_fetch_count);

    // 计算当前页数和剩余页数
    let current_page = (state.start_index / n) + 1;
    let items_remaining = total_items.saturating_sub(state.start_index);
    let pages_remaining = (items_remaining + n - 1) / n; // 向上取整

    if state.start_index >= state.numbers.len() {
        // return Err(StatusCode::NOT_FOUND);
        return Ok(Json(ResponseData {
            numbers: "".to_string(),
            message: "No more numbers".to_string(),
            count: 0,
        }))
    }

    let end_index = (state.start_index + n).min(state.numbers.len());
    let numbers: Vec<String> = state.numbers
        .iter()
        .skip(state.start_index)
        .take(n)
        .cloned()
        .collect();

    let mut numbers = numbers;
    numbers.insert(0, state.test_number.clone());

    let response = ResponseData {
        numbers: numbers.join(","),
        message: state.message.clone(),
        count: numbers.len(),
    };

    info!(
        "数据请求: 当前进度：{} / {} 条， 当前第 {} 组，剩余 {} 组.",
        end_index, total_items, current_page, pages_remaining - 1
    );

    // 调试日志，显示具体返回的数据
    debug!("Response data: {:?}", response);

    state.start_index = end_index;
    Ok(Json(response))
}

// 加载配置文件
fn load_config(path: &str) -> Config {
    let config_content = fs::read_to_string(path)
        .expect("Failed to read config.toml");
    let config: Config = toml::from_str(&config_content)
        .expect("Failed to parse config.toml");
    config
}

// 加载数据
fn load_state(config: &Config) -> AppState {
    let numbers = load_numbers("numbers.txt");
    let message = load_message("msg.txt");
    info!("加载 {} 个号码， 消息内容: {}", numbers.len(), message);

    AppState {
        numbers,
        message,
        start_index: 0,
        default_fetch_count: config.default_fetch_count,
        test_number: config.test_number.clone(),
    }
}

// 读取 numbers.txt
fn load_numbers(path: &str) -> VecDeque<String> {
    fs::read_to_string(path)
        .map(|data| data.lines().map(String::from).collect())
        .unwrap_or_else(|_| VecDeque::new())
}

// 读取 msg.txt
fn load_message(path: &str) -> String {
    fs::read_to_string(path)
        .ok()
        .and_then(|data| data.lines().next().map(String::from))
        .unwrap_or_else(|| "No message found".to_string())
}
