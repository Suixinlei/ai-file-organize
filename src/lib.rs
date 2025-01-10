use std::fs;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use reqwest::Client;

#[derive(Deserialize, Debug)]
struct Classification {
    prompt: String,
    dir: String,
}

#[derive(Deserialize, Debug)]
struct AppConfig {
    openai_endpoint: String,
    openai_api_key: String,
    openai_model: String,
    classifications: Vec<Classification>,
}

pub async fn run_app(temp_dir: String, config_path: Option<String>, ) -> Result<(), Box<dyn std::error::Error>> {
  println!("temp_dir: {}", temp_dir);
  // 1. 读取配置文件
  let app_config: AppConfig = load_config(&config_path.unwrap_or_else(|| "config.json".into()))?;

  // 2. 创建 HTTP 客户端
  let client = Client::new();

  // 3. 获取所有子文件夹(depth=1)
  let sub_folders = get_subfolders(&temp_dir)?;

  println!("sub_folders: {:?}", sub_folders);

  for folder in sub_folders {
    // 4. 分析这个子文件夹下的文件信息
    let folder_info = analyze_folder(&folder)?;

    // 5. 调用 GPT，获取分类结果
    let category = match classify_folder_with_openai(
      &client,
      &app_config.openai_endpoint,
      &app_config.openai_api_key,
      &app_config.openai_model,
      &folder_info,
    )
    .await
    {
      Ok(category) => category,
      Err(e) => {
        eprintln!("Failed to classify folder {}: {}", folder.display(), e);
        continue;
      }
    };

    // 6. 根据 category， 在配置中找到目标路径
    // 如果找不到，就用 others 兜底
    let target_path = app_config.classifications.iter()
        .find(|c| c.prompt == category)
        .map(|c| &c.dir)
        .expect("No matching category found in config");

    // 7. 将文件夹移动到目标路径
    move_folder(&folder, Path::new(target_path))?;
  }

  // 8. 读取所有一级文件
  let files = get_files(&temp_dir)?;
  for file in files {
    let file_info = analyze_file(&file)?;

    let category = match classify_folder_with_openai(
      &client,
      &app_config.openai_endpoint,
      &app_config.openai_api_key,
      &app_config.openai_model,
      &file_info,
    )
    .await
    {
      Ok(cat) => cat,
      Err(e) => {
        eprintln!("Failed to classify folder {}: {}", file_info, e);
        continue;
      }
    };

    let target_path = app_config.classifications.iter()
        .find(|c| c.prompt == category)
        .map(|c| &c.dir)
        .expect("No matching category found in config");

    move_folder(&file, Path::new(target_path))?;
  }
  

  Ok(())
}

fn load_config(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
  println!("load_config: {}", path);
  let content = fs::read_to_string(path)?;
  let app_config:AppConfig = serde_json::from_str(&content)?;
  Ok(app_config)
}

fn get_files(dir: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
  let mut result = Vec::new();
  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();
    if path.is_file() {
      result.push(entry.path());
    }
  }
  Ok(result)
}

fn analyze_file(file_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
  Ok(file_path.file_name().unwrap().to_string_lossy().to_string())
}

fn get_subfolders(dir: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
  let mut result = Vec::new();
  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();
    if path.is_dir() {
      result.push(entry.path());
    }
  }
  Ok(result)
}

// 分析文件夹，获取一些特征信息，可以是文件名列表、大小等等
fn analyze_folder(folder_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
  let mut file_names = Vec::new();
  for entry in fs::read_dir(folder_path)? {
    let entry = entry?;
    if entry.path().is_file() {
      file_names.push(entry.file_name().to_string_lossy().to_string());
    }
    
    // TODO: 还可以加上文件大小、类型等信息
  }

  // 这里我们简单地返回一个逗号分隔的文件名列表
  Ok(file_names.join(","))
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
