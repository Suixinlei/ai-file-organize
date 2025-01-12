use std::fs;
use std::path::{PathBuf};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Classification {
    pub prompt: String,
    pub dir: String,
}

#[derive(Deserialize, Debug)]
pub struct AppConfig {
  pub classifications: Vec<Classification>,
}


pub struct LoadConfigResult {
    pub app_config: AppConfig,
    pub sub_files: Vec<PathBuf>
}

fn get_subfiles(dir: &str, destination_dirs: &Vec<String>) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
  let mut result = Vec::new();

  for entry in fs::read_dir(dir)? {
    let entry = entry?;
    let path = entry.path();
    // 获取完整绝对路径
    let full_path = path.to_string_lossy().to_string();

    // 如果是 .DS_Store 文件则跳过
    if path.file_name().unwrap().to_string_lossy() == ".DS_Store" {
      continue;
    }

    // 如果目标文件夹列表中包含这个文件夹，则跳过 
    if destination_dirs.contains(&full_path) {
      continue;
    }

    result.push(path);
  }
  Ok(result)
}

pub fn load_config(temp_dir: &str, config_path: &str) -> Result<LoadConfigResult, Box<dyn std::error::Error>> {
  // 获取实际配置
  println!("load_config: {}", config_path);
  let content = fs::read_to_string(config_path)?;
  let app_config:AppConfig = serde_json::from_str(&content)?;

  let destination_dirs = app_config.classifications.iter().map(|c| c.dir.clone()).collect::<Vec<String>>();
  
  // 获取 temp_dir 下的所有目录
  let sub_files = get_subfiles(temp_dir, &destination_dirs)?;
  
  Ok(LoadConfigResult {
    sub_files,
    app_config,
  })
}
