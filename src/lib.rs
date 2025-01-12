mod load_config;

use load_config::load_config;
use std::fs;
use std::path::{Path};
use chrono;
use walkdir::WalkDir;  


use reqwest::Client;

pub async fn run_app(temp_dir: String, config_path: Option<String>, ) -> Result<(), Box<dyn std::error::Error>> {
  let config_path_str = config_path.unwrap_or("config.json".to_string());
  let load_config_result = load_config(&temp_dir, &config_path_str)?;

  let app_config = load_config_result.app_config;

  println!("app_config: {:?}", app_config);

  let sub_files = load_config_result.sub_files;

  println!("sub_files: {:?}", sub_files);

  for file in sub_files {

    let file_info = analyze_file(&file)?;

    println!("file_info: {}", file_info);
  }
  

  Ok(())
}

fn print_tree_walkdir(path: &str) -> Result<String, Box<dyn std::error::Error>> {
  let mut tree_output = Vec::new();
  for entry in WalkDir::new(path).max_depth(1).into_iter().filter_map(|e| e.ok()) {  
      let depth = entry.depth();  
      let indent = "    ".repeat(depth);  
      tree_output.push(format!("{}└── {}", indent, entry.file_name().to_string_lossy()));
  }  
  Ok(tree_output.join("\n"))
}

fn analyze_file(file_path: &Path) -> Result<String, Box<dyn std::error::Error>> {

  // 检测是否为目录
  if file_path.is_dir() {
    let tree_output = print_tree_walkdir(&file_path.to_string_lossy())?;
    return Ok(tree_output);
  }

  // 如果是文件，需要获取
  // 1. 文件名
  // 2. 文件大小, 使用 KB 单位
  // 3. 文件创建日期, 使用 yyyy-MM-dd HH:mm:ss 格式
  // 4. 文件修改日期, 使用 yyyy-MM-dd HH:mm:ss 格式
  let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
  let file_size = (file_path.metadata()?.len() as f64 / 1024.0).round() / 100.0;
  let file_create_date = file_path.metadata()?.created()?;
  let file_modify_date = file_path.metadata()?.modified()?;
  
  // 格式化日期
  let file_create_date = chrono::DateTime::<chrono::Local>::from(file_create_date)
    .format("%Y-%m-%d %H:%M:%S")
    .to_string();
  let file_modify_date = chrono::DateTime::<chrono::Local>::from(file_modify_date)
    .format("%Y-%m-%d %H:%M:%S")
    .to_string();

  // 将以上信息合并到一段文字中
  let file_info = format!("文件名: {}, 文件大小: {} KB, 文件创建日期: {}, 文件修改日期: {}", file_name, file_size, file_create_date, file_modify_date);

  Ok(file_info)
}

// 调用 OpenAI API 进行分类
async fn classify_folder_with_openai(
  client: &Client,
  endpoint: &str,
  api_key: &str,
  model: &str,
  folder_info: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  // 增加调用的调试信息
  println!("classify_folder_with_openai: endpoint: {}", endpoint);
  println!("classify_folder_with_openai: api_key: {}", api_key);
  println!("classify_folder_with_openai: model: {}", model);
  println!("classify_folder_with_openai: folder_info: {}", folder_info);

  let prompt = format!(r#"请从下列类型中选择一个最合适的类型并只返回这个 type 的 JSON, 例如 {{ "category": "movie" }}. 已知类型: ["movie", "anime", "document", "others"]。 文件夹内容信息: {}"#, folder_info);
  let body = serde_json::json!({
    "model": model,
    "messages": [
        {
            "role": "system",
            "content": "你是一个分类助手"
        },
        {
            "role": "user",
            "content": prompt
        }
    ]
  });

  let resp = client
      .post(endpoint)
      .bearer_auth(api_key)
      .json(&body)
      .send()
      .await?
      .error_for_status()?   // 如果返回错误，会报错
      .json::<serde_json::Value>()
      .await?;

  // 简单解析: 假设我们引导 GPT 只返回一行 JSON: { "category": "<xxx>" }
  // 下面要从返回的 JSON 中取到 GPT 从 assistant role 对话中的content
  if let Some(choices) = resp["choices"].as_array() {
      if let Some(choice) = choices.get(0) {
          if let Some(content) = choice["message"]["content"].as_str() {
              // 对content进行解析
              let parsed: serde_json::Value = serde_json::from_str(content)?;
              if let Some(cat) = parsed["category"].as_str() {
                  return Ok(cat.to_string());
              }
          }
      }
  }

  Ok("others".to_string())

}

// 移动文件夹
fn move_folder(folder_path: &Path, target_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
  // 先确认目标文件夹是否存在，不存在则创建
  if !target_path.exists() {
    fs::create_dir_all(target_path)?;
  }
  // 构建新的路径
  let folder_name = folder_path.file_name().unwrap();
  let new_path = target_path.join(folder_name);
  // 使用 rename, 如果跨分区或磁盘，需要用别的方法复制文件再删除
  fs::rename(folder_path, new_path)?;
  Ok(())
}
