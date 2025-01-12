
use std::path::{Path};
use chrono;
use walkdir::WalkDir;  

fn print_tree_walkdir(path: &str) -> Result<String, Box<dyn std::error::Error>> {
  let mut tree_output = Vec::new();
  for entry in WalkDir::new(path).max_depth(1).into_iter().filter_map(|e| e.ok()) {  
      let depth = entry.depth();  
      let indent = "    ".repeat(depth);  
      tree_output.push(format!("{}└── {}", indent, entry.file_name().to_string_lossy()));
  }  
  Ok(tree_output.join("\n"))
}

pub fn analyze_file(file_path: &Path) -> Result<String, Box<dyn std::error::Error>> {

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