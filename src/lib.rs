mod load_config;
mod analyze_file;

use load_config::load_config;
use analyze_file::analyze_file;
use std::fs;
use std::path::{Path};
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

    let category = classify_folder_with_openai(&file_info).await?;

    println!("category: {}", category);
  }
  

  Ok(())
}

// 调用 OpenAI API 进行分类
async fn classify_folder_with_openai(
  file_info: &str,
) -> Result<String, Box<dyn std::error::Error>> {
  let client = Client::new();
  let body = serde_json::json!({
    "input": {
      "prompt": file_info,
    }
  });

  let resp = client
      .post("https://dashscope.aliyuncs.com/api/v1/apps/f26835a3c89d447786d0e8483a96e90f/completion")
      .bearer_auth("sk-1fa68db6df854138b74224e20d5b5e20	")
      .json(&body)
      .send()
      .await?
      .error_for_status()?   // 如果返回错误，会报错
      .json::<serde_json::Value>()
      .await?;

  println!("resp: {:?}", resp);

  // 获取 resp.output.text
  let category = resp["output"]["text"].as_str().unwrap_or("others");

  Ok(category.to_string())

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
