use std::env;
use std::fs;
use std::io::BufReader;
use std::io::Read;
use std::path;
use std::path::Path;
use std::path::PathBuf;

// 定义节点枚举：目录（含子节点）或 文件（含可选注释）
#[derive(Debug)]
enum Node {
    Dir(String, Vec<Node>), // 目录：名称 + 子节点列表
    File(String),           // 文件：名称 + 注释（自动遍历无注释，设为 None）
}

impl Node {
    fn is_dir(&self) -> bool {
        matches!(self, Self::Dir(_, _))
    }

    /// 递归打印目录树（核心：处理前缀、连接线、层级）
    fn print(&self, prefix: &str, is_last: bool) {
        match self {
            // 打印目录：先打印自身，再递归打印子节点
            Node::Dir(name, children) => {
                self.print_single_node(prefix, is_last, name);
                // 遍历子节点，计算子节点的前缀和是否为最后一个
                for (index, child) in children.iter().enumerate() {
                    let child_is_last = index == children.len() - 1;
                    // // 父节点非最后一个 → 子节点前缀加「│  」，否则加「   」（保持竖线对齐）
                    // let child_prefix = if is_last {
                    //     format!("{prefix}   ")
                    // } else {
                    //     format!("{prefix}│  ")
                    // };
                    child.print(&format!("{prefix}   "), child_is_last);
                }
            }
            // 打印文件：直接打印名称（无注释）
            Node::File(name) => {
                self.print_single_node(prefix, is_last, name);
            }
        }
    }

    /// 辅助函数：打印单个节点（拼接前缀、连接线、名称、注释）
    fn print_single_node(&self, prefix: &str, is_last: bool, name: &str) {
        // 最后一个节点用「└─」，否则用「├─」
        let connector = if is_last { "└─ " } else { "├─ " };
        let output = format!("{prefix}{connector}{name}");
        println!("{}", output);
    }
}

/// 递归构建目录树：从指定路径读取文件/目录，生成 Node 结构
fn build_tree(path: &Path, ignores: &Vec<String>) -> Option<Node> {
    if let Some(name) = path.file_name().and_then(|n| n.to_str())
        && ignores.iter().any(|ig| ig == name)
    {
        return None;
    }
    // 获取节点名称（路径最后一段，如 "src" "Cargo.toml"）
    let node_name = path
        .file_name()
        .expect("无法获取路径名称")
        .to_str()
        .expect("路径名称非 UTF-8 编码")
        .to_string();

    // 路径是目录 → 递归读取子节点
    if path.is_dir() {
        let mut children = Vec::new();
        // 读取目录下所有项（过滤 . 和 ..，避免无限递归）
        let entries = fs::read_dir(path).expect("读取目录失败，请检查权限或路径");
        for entry in entries {
            let entry = entry.expect("读取目录项失败");
            let entry_path = entry.path();
            // 跳过隐藏文件/目录（可选，可根据需求删除）
            if let Some(name) = entry_path.file_name().and_then(|s| s.to_str())
                && name.starts_with(".")
            {
                continue;
            }
            // 递归构建子节点
            if let Some(sub) = build_tree(&entry_path, ignores) {
                children.push(sub);
            }
        }
        children.sort_by(|a, b| match (a, b) {
            (Node::Dir(an, _), Node::Dir(bn, _)) => an.cmp(bn),
            (Node::File(an), Node::File(bn)) => an.cmp(bn),
            (Node::Dir(an, _), _) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Greater,
        });
        Some(Node::Dir(node_name, children))
    } else {
        // 路径是文件 → 生成文件节点（无注释）
        Some(Node::File(node_name))
    }
}

fn find_gitignore(path: &Path) -> Vec<String> {
    if path.is_dir() {
        let ignore_path = path.join(".gitignore");
        if ignore_path.exists() && ignore_path.is_file() {
            let mut data = Vec::new();
            let ignore_file = fs::File::open(ignore_path).expect("no ignore path");
            let mut buf_reader = BufReader::new(ignore_file);
            buf_reader
                .read_to_end(&mut data)
                .expect("read content failed for gitignore");
            return String::from_utf8(data)
                .expect("not utf8")
                .split("\n")
                .map(|s| s.to_owned())
                .collect();
        }
    }

    Vec::new()
}

fn main() {
    // 获取当前工作目录（程序运行的目录）
    let dir = env::current_dir().expect("获取当前目录失败");
    // println!("当前目录：{}\n目录树结构：", current_dir.display());

    // let dir: PathBuf = "../".into();
    let mut ignores = find_gitignore(&dir);
    ignores.extend_from_slice(&["Cargo.lock".to_owned(), "bin".to_owned()]);
    // 递归构建目录树
    let tree = build_tree(&dir, &ignores).expect("no tree");
    // println!("tree: {:?}", tree);

    // 打印根节点（初始前缀为空，根节点是唯一节点，设为最后一个）
    tree.print("", true);
}
