// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#![allow(unused)]
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::sync::Mutex;
use uuid::Uuid;

use xmltree::Element;

use sxd_document::parser as sxd_parser;
use sxd_xpath::{Context, Factory, Value};

use std::{collections::HashSet, path::Path, process::Command};
use tauri::Manager;

/// CachedDoc: 各ファイルごとのキャッシュ
struct CachedDoc {
    path: PathBuf,
    dom: Element, // editable DOM (xmltree)
    text: String, // latest serialized text
    // node map: node_id -> pseudo-xpath (we use indexes)
    node_map: HashMap<String, String>,
}

/// グローバルキャッシュ（簡易）
static CACHE: Lazy<Mutex<HashMap<String, CachedDoc>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize)]
struct FileSummary {
    path: String,
    project_name: Option<String>,
}

#[derive(Serialize)]
struct TreeNode {
    key: String,
    label: String,
    children: Vec<TreeNode>,
    node_type: String,
}

#[tauri::command]
async fn load_files(paths: Vec<String>) -> Result<Vec<FileSummary>, String> {
    let mut results = Vec::new();
    let mut cache = CACHE.lock().await;

    for p in paths {
        let pathbuf = PathBuf::from(&p);
        let text = fs::read_to_string(&pathbuf).map_err(|e| format!("read error: {}", e))?;
        // parse via xmltree
        let root =
            Element::parse(text.as_bytes()).map_err(|e| format!("xml parse error: {}", e))?;
        // create cache entry
        let doc = CachedDoc {
            path: pathbuf.clone(),
            dom: root,
            text: text.clone(),
            node_map: HashMap::new(),
        };
        cache.insert(p.clone(), doc);

        // try to extract project/name for quick summary
        let project_name = extract_project_name_from_text(&text).ok();

        results.push(FileSummary {
            path: p.clone(),
            project_name,
        });
    }

    Ok(results)
}

/// Helper: try to quickly extract /project/name from raw text using sxd xpath
fn extract_project_name_from_text(text: &str) -> Result<String, String> {
    let package = sxd_parser::parse(text).map_err(|e| format!("sxd parse err {:?}", e))?;
    let doc = package.as_document();
    let factory = Factory::new();
    let xpath = factory
        .build("/project/name/text()")
        .map_err(|e| format!("xpath build {:?}", e))?
        .ok_or("compile failed")?;
    let context = Context::new();
    let value = xpath
        .evaluate(&context, doc.root())
        .map_err(|e| format!("evaluate {:?}", e))?;
    match value {
        Value::Nodeset(ns) => {
            for node in ns.document_order() {
                return Ok(node.string_value());
            }
            Err("no node".into())
        }
        v => Ok(v.string()),
    }
}

/// Build tree JSON for client (following the 4-level specification)
#[tauri::command]
async fn get_tree(path: String) -> Result<Vec<TreeNode>, String> {
    let mut cache = CACHE.lock().await;
    let entry = cache.get_mut(&path).ok_or("not loaded")?;

    // clear node_map (we will repopulate with fresh ids)
    entry.node_map.clear();
    let root = &entry.dom;

    // Expect root element "project"
    // 1st level: file node -> label "/project/name"
    // We'll create one root node representing the file, then children for targets, groups, files.
    let root_label = format!(
        "/project/{}",
        get_child_text(root, "name").unwrap_or_else(|| "<no-name>".into())
    );
    let root_id = Uuid::new_v4().to_string();
    entry
        .node_map
        .insert(root_id.clone(), "/project".to_string());

    // children: targets
    let mut targets_nodes = Vec::new();
    for (t_idx, target_el) in get_children_by_name(root, "target").into_iter().enumerate() {
        // pseudo xpath: /project/target[t_idx+1]
        let t_xpath = format!("/project/target[{}]", t_idx + 1);
        let t_id = Uuid::new_v4().to_string();
        let t_label = format!("{}/name", t_xpath);
        // store mapping
        entry.node_map.insert(t_id.clone(), t_xpath.clone());

        // groups under target
        let mut groups_nodes = Vec::new();
        if let Some(groups_el) = find_child_element(&target_el, "groups") {
            for (g_idx, group_el) in get_children_by_name(groups_el, "group")
                .into_iter()
                .enumerate()
            {
                let g_xpath = format!("{} /groups/group[{}]", t_xpath, g_idx + 1).replace(" ", "");
                let g_id = Uuid::new_v4().to_string();
                let g_name =
                    get_child_text(&group_el, "name").unwrap_or_else(|| "<no-name>".into());
                entry.node_map.insert(g_id.clone(), g_xpath.clone());

                // files under group
                let mut file_nodes = Vec::new();
                if let Some(files_el) = find_child_element(&group_el, "files") {
                    for (f_idx, file_el) in get_children_by_name(files_el, "file")
                        .into_iter()
                        .enumerate()
                    {
                        let f_xpath = format!("{}/files/file[{}]", g_xpath, f_idx + 1);
                        let f_id = Uuid::new_v4().to_string();
                        let f_name =
                            get_child_text(&file_el, "name").unwrap_or_else(|| "<no-name>".into());
                        entry.node_map.insert(f_id.clone(), f_xpath.clone());

                        file_nodes.push(TreeNode {
                            key: f_id,
                            label: f_name,
                            children: vec![],
                            node_type: "file".into(),
                        });
                    }
                }

                groups_nodes.push(TreeNode {
                    key: g_id,
                    label: g_name,
                    children: file_nodes,
                    node_type: "group".into(),
                });
            }
        }

        targets_nodes.push(TreeNode {
            key: t_id,
            label: get_child_text(&target_el, "name").unwrap_or_else(|| "<no-name>".into()),
            children: groups_nodes,
            node_type: "target".into(),
        });
    }

    let tree = vec![TreeNode {
        key: root_id,
        label: root_label,
        children: targets_nodes,
        node_type: "project".into(),
    }];

    Ok(tree)
}

/// Sort files inside each group node specified by node_ids (which refer to group nodes)
#[tauri::command]
async fn sort_groups(node_ids: Vec<String>, ascending: bool) -> Result<(), String> {
    let mut cache = CACHE.lock().await;
    // find which cached doc contains each node id
    for (path, entry) in cache.iter_mut() {
        for nid in node_ids.iter() {
            if let Some(xpath) = entry.node_map.get(nid) {
                // only handle group nodes (xpath containing /groups/group[..])
                if !xpath.contains("/groups/group[") {
                    continue;
                }
                // locate group element in entry.dom using our pseudo-xpath
                if let Some((parent_files_el, mut file_elems)) =
                    find_group_files_mut(&mut entry.dom, xpath)
                {
                    // sort file_elems by <name> text
                    file_elems.sort_by(|a, b| {
                        let na = get_child_text(a, "name").unwrap_or_default();
                        let nb = get_child_text(b, "name").unwrap_or_default();
                        if ascending {
                            na.cmp(&nb)
                        } else {
                            nb.cmp(&na)
                        }
                    });
                    // rebuild parent's <files> children from sorted file_elems
                    parent_files_el
                        .children
                        .retain(|c| !is_element_named(c, "file"));
                    for fe in file_elems {
                        parent_files_el
                            .children
                            .push(xmltree::XMLNode::Element(fe.clone()));
                    }
                }
            }
        }
        // after edits, update entry.text to serialized xml
        entry.text = serialize_element(&entry.dom)?;
    }
    Ok(())
}

/// Add a new <file><name>file_name</name></file> to each group node in node_ids
#[tauri::command]
async fn add_file_to_groups(node_ids: Vec<String>, file_name: String) -> Result<(), String> {
    let mut cache = CACHE.lock().await;
    for (_path, entry) in cache.iter_mut() {
        for nid in node_ids.iter() {
            if let Some(xpath) = entry.node_map.get(nid) {
                if !xpath.contains("/groups/group[") {
                    continue;
                }
                if let Some((_files_el, _file_elems)) = find_group_files_mut(&mut entry.dom, xpath)
                {
                    // find files element mutable and push a new file element
                    if let Some((files_el, _)) = find_group_files_mut(&mut entry.dom, xpath) {
                        let mut new_file = Element::new("file");
                        let mut name_el = Element::new("name");
                        name_el
                            .children
                            .push(xmltree::XMLNode::Text(file_name.clone()));
                        new_file.children.push(xmltree::XMLNode::Element(name_el));
                        files_el.children.push(xmltree::XMLNode::Element(new_file));
                    }
                }
            }
        }
        entry.text = serialize_element(&entry.dom)?;
    }
    Ok(())
}

/// Delete file nodes specified by node_ids (these should be file-level node ids)
#[tauri::command]
async fn delete_file_nodes(node_ids: Vec<String>) -> Result<(), String> {
    let mut cache = CACHE.lock().await;
    for (_path, entry) in cache.iter_mut() {
        for nid in node_ids.iter() {
            if let Some(xpath) = entry.node_map.get(nid) {
                if !xpath.contains("/files/file[") {
                    continue;
                }
                // remove the file element by adjusting parent's children
                remove_file_by_xpath(&mut entry.dom, xpath)?;
            }
        }
        entry.text = serialize_element(&entry.dom)?;
    }
    Ok(())
}

/// Save cache for a given file path to disk
#[tauri::command]
async fn save_file(path: String) -> Result<(), String> {
    let mut cache = CACHE.lock().await;
    let entry = cache.get_mut(&path).ok_or("not loaded")?;
    // entry.text should be up-to-date
    fs::write(&entry.path, &entry.text).map_err(|e| format!("write err: {}", e))?;
    Ok(())
}

fn serialize_element(el: &Element) -> Result<String, String> {
    // write to memory (xmltree has write() but easier to use to_string)
    let mut vec = Vec::new();
    el.write(&mut vec)
        .map_err(|e| format!("serialize err: {}", e))?;
    String::from_utf8(vec).map_err(|e| format!("utf8 err: {}", e))
}

/// Utility functions for xmltree traversal

fn get_child_text<'a>(el: &'a Element, name: &str) -> Option<String> {
    find_child_element(el, name).and_then(|c| {
        // first text child or concatenated text nodes
        let mut s = String::new();
        for child in c.children.iter() {
            if let xmltree::XMLNode::Text(t) = child {
                s.push_str(t);
            }
        }
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    })
}

fn find_child_element<'a>(el: &'a Element, name: &str) -> Option<&'a Element> {
    for child in el.children.iter() {
        if let xmltree::XMLNode::Element(e) = child {
            if e.name == name {
                return Some(e);
            }
        }
    }
    None
}

/// get children elements with specific name (owned clones to avoid borrow issues)
fn get_children_by_name(el: &Element, name: &str) -> Vec<Element> {
    el.children
        .iter()
        .filter_map(|c| {
            if let xmltree::XMLNode::Element(e) = c {
                if e.name == name {
                    Some(e.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// check xmltree::XMLNode is element with name
fn is_element_named(node: &xmltree::XMLNode, name: &str) -> bool {
    if let xmltree::XMLNode::Element(e) = node {
        e.name == name
    } else {
        false
    }
}

/// Find the <files> Element and return (&mut files_el, Vec<Element> current file elements)
/// xpath sample: /project/target[1]/groups/group[2]
fn find_group_files_mut<'a>(
    root: &'a mut Element,
    xpath: &str,
) -> Option<(&'a mut Element, Vec<Element>)> {
    // parse xpath segments
    // break by '/'
    let segs: Vec<&str> = xpath.trim_matches('/').split('/').collect();
    // we expect segs like ["project","target[1]","groups","group[2]"]
    // we'll traverse from root
    let mut cur: *mut Element = root as *mut _;
    // for seg in segs.iter().skip(1) {

    for seg in segs.iter().copied() {
        // skip initial "project"
        // parse name and optional [n]
        let (name, idx) = if let Some(start) = seg.find('[') {
            let name = &seg[0..start];
            let idx_s = seg[start + 1..seg.len() - 1].to_string();
            (name, idx_s.parse::<usize>().ok().unwrap_or(1))
        } else {
            (seg, 1usize)
        };
        // move cur to child element name with index idx (1-based)
        unsafe {
            let cur_ref: &mut Element = &mut *cur;
            let mut count = 0usize;
            let mut found_ptr: *mut Element = std::ptr::null_mut();
            for child in cur_ref.children.iter_mut() {
                if let xmltree::XMLNode::Element(e) = child {
                    if e.name == name {
                        count += 1;
                        if count == idx {
                            found_ptr = e as *mut _;
                            break;
                        }
                    }
                }
            }
            if found_ptr.is_null() {
                return None;
            }
            cur = found_ptr;
        }
    }
    // now cur points to the group element
    unsafe {
        let group_el: &mut Element = &mut *cur;
        if let Some(files_child) = group_el.children.iter_mut().find_map(|c| {
            if let xmltree::XMLNode::Element(e) = c {
                if e.name == "files" {
                    Some(e as *mut Element)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            let files_el: &mut Element = &mut *files_child;
            // collect file elements clones
            let file_elems: Vec<Element> = files_el
                .children
                .iter()
                .filter_map(|c| {
                    if let xmltree::XMLNode::Element(e) = c {
                        if e.name == "file" {
                            Some(e.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            return Some((files_el, file_elems));
        } else {
            // create one if not exists
            let mut new_files = Element::new("files");
            group_el.children.push(xmltree::XMLNode::Element(new_files));
            // now find it again
            if let Some(files_child) = group_el.children.iter_mut().find_map(|c| {
                if let xmltree::XMLNode::Element(e) = c {
                    if e.name == "files" {
                        Some(e as *mut Element)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }) {
                let files_el: &mut Element = &mut *files_child;
                let file_elems: Vec<Element> = vec![];
                return Some((files_el, file_elems));
            }
        }
    }
    None
}

/// Remove file by pseudo-xpath like ".../files/file[2]"
fn remove_file_by_xpath(root: &mut Element, xpath: &str) -> Result<(), String> {
    // find parent files element and index to remove
    if let Some(pos) = xpath.rfind("/files/file[") {
        let parent_path = &xpath[..pos + 7]; // includes /files
                                             // parse index
        if let Some(start) = xpath[pos + 12..].find(']') {
            // actually simpler: extract last [n]
        }
    }
    // simpler approach: traverse to files element and remove the indexed child
    // get segments
    let segs: Vec<&str> = xpath.trim_matches('/').split('/').collect();
    // last segment is files/file[n], we want files element and index
    if segs.len() < 1 {
        return Err("invalid xpath".into());
    }
    let last = segs.last().unwrap();
    // last should be like files/file[n] or file[n]
    // find files element path by removing trailing /file[n]
    let mut files_segs = segs.clone();
    files_segs.pop(); // remove file[n]
                      // now find files element
    let mut cur: *mut Element = root as *mut _;
    // for seg in files_segs.iter().skip(1) {
    for seg in files_segs.iter().copied() {
        // skip project
        let (name, idx) = if let Some(start) = seg.find('[') {
            let name = &seg[0..start];
            let idx_s = seg[start + 1..seg.len() - 1].to_string();
            (name, idx_s.parse::<usize>().ok().unwrap_or(1))
        } else {
            (seg, 1usize)
        };
        unsafe {
            let cur_ref: &mut Element = &mut *cur;
            let mut count = 0usize;
            let mut found_ptr: *mut Element = std::ptr::null_mut();
            for child in cur_ref.children.iter_mut() {
                if let xmltree::XMLNode::Element(e) = child {
                    if e.name == name {
                        count += 1;
                        if count == idx {
                            found_ptr = e as *mut _;
                            break;
                        }
                    }
                }
            }
            if found_ptr.is_null() {
                return Err("not found parent".into());
            }
            cur = found_ptr;
        }
    }

    // now cur is files element
    // parse file index from last seg
    let seg = *segs.last().unwrap();
    let idx = if let Some(start) = seg.find('[') {
        seg[start + 1..seg.len() - 1]
            .parse::<usize>()
            .map_err(|_| "bad index")?
    } else {
        1usize
    };

    unsafe {
        let files_el: &mut Element = &mut *cur;
        // find nth file element position among children
        let mut count = 0usize;
        let mut remove_pos: Option<usize> = None;
        for (i, child) in files_el.children.iter().enumerate() {
            if let xmltree::XMLNode::Element(e) = child {
                if e.name == "file" {
                    count += 1;
                    if count == idx {
                        remove_pos = Some(i);
                        break;
                    }
                }
            }
        }
        if let Some(pos) = remove_pos {
            files_el.children.remove(pos);
            return Ok(());
        } else {
            return Err("file not found".into());
        }
    }
}

#[derive(Debug, Serialize)]
struct TreeIncludeNode {
    id: String,
    label: String,
    full_path: String, // ← 追加
    children: Vec<TreeIncludeNode>,
    exists: bool,
    selected: bool,
    inside_repo: bool,
    registed: HashMap<String, bool>,
}

/// 1. Gitリポジトリのルートを探す
fn find_git_root(start: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(start)
        .output()
        .ok()?;
    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(PathBuf::from(path_str))
    } else {
        None
    }
}

/// 3. 共通の親ディレクトリを探す（repo_root以下）
fn common_parent<'a>(repo_root: &Path, paths: &[&'a Path]) -> PathBuf {
    let mut components: Vec<Vec<&std::ffi::OsStr>> = paths
        .iter()
        .map(|p| p.components().map(|c| c.as_os_str()).collect())
        .collect();

    if components.is_empty() {
        return repo_root.to_path_buf();
    }

    let mut common: Vec<&std::ffi::OsStr> = vec![];
    'outer: for i in 0.. {
        let first = match components[0].get(i) {
            Some(c) => c,
            None => break,
        };
        for comp in &components {
            if comp.get(i) != Some(first) {
                break 'outer;
            }
        }
        common.push(first);
    }

    let mut result = PathBuf::new();
    for c in common {
        result.push(c);
    }
    if result.starts_with(repo_root) {
        result
    } else {
        repo_root.to_path_buf()
    }
}

/// 4. 再帰的にフォルダツリーを走査（リポジトリ内）
fn build_tree(base: &Path, repo_root: &Path, all_selected: &HashSet<PathBuf>) -> TreeIncludeNode {
    let exists = base.exists();
    let selected = all_selected.contains(base);
    let inside_repo = base.starts_with(repo_root);

    let mut children = vec![];

    // 1. 存在するディレクトリなら通常の fs::read_dir を走査
    if exists && base.is_dir() {
        if let Ok(entries) = fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                children.push(build_tree(&path, repo_root, all_selected));
            }
        }
    }

    // 2. 存在しない場合でも all_selected の中に base 以下のパスがある場合は子を生成
    for sel in all_selected.iter() {
        if sel.starts_with(base) && sel != base {
            let mut components = sel.strip_prefix(base).unwrap().components();
            if let Some(next_comp) = components.next() {
                let next_path = base.join(next_comp.as_os_str());
                // 重複チェック
                if !children
                    .iter()
                    .any(|c: &TreeIncludeNode| c.full_path == next_path.to_string_lossy())
                {
                    children.push(build_tree(&next_path, repo_root, all_selected));
                }
            }
        }
    }

    let mut registed = HashMap::new();
    registed.insert("arrayA".to_string(), false);
    registed.insert("arrayB".to_string(), false);
    registed.insert("arrayC".to_string(), false);

    TreeIncludeNode {
        id: base.to_string_lossy().to_string(),
        label: if inside_repo {
            base.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        } else {
            base.to_string_lossy().to_string()
        },
        full_path: base.to_string_lossy().to_string(),
        children,
        exists,
        selected,
        inside_repo,
        registed,
    }
}

/// リポジトリ外ノードを単独で生成
fn build_external_node(path: &Path, all_selected: &HashSet<PathBuf>) -> TreeIncludeNode {
    let mut registed = HashMap::new();
    registed.insert("arrayA".to_string(), false);
    registed.insert("arrayB".to_string(), false);
    registed.insert("arrayC".to_string(), false);
    TreeIncludeNode {
        id: path.to_string_lossy().to_string(),
        label: path.to_string_lossy().to_string(), // ← repo外はフルパスをそのままlabelに
        full_path: path.to_string_lossy().to_string(),
        children: vec![],
        exists: path.exists(),
        selected: all_selected.contains(path),
        inside_repo: false,
        registed: registed,
    }
}

#[tauri::command]
async fn get_include_tree_nodes() -> Result<Vec<TreeIncludeNode>, String> {
    // 仮の入力
    let known_file =
        PathBuf::from("C:\\Users\\Admin\\Desktop\\script\\aaaa\\src-tauri\\.gitignore");
    let given_folders = vec![
        PathBuf::from("C:\\Users\\Admin\\Desktop\\script\\aaaa\\inc\\a\\b"),
        PathBuf::from("C:\\Users\\Admin\\Desktop\\script\\aaaa\\inc\\g\\h"),
        PathBuf::from("C:\\Users\\Admin\\Desktop\\script\\yamp-test\\src"),
    ];

    // 1. gitリポジトリルート探索
    let repo_root = find_git_root(&known_file.parent().unwrap()).expect("not in git repo");

    // 2. gitリポジトリ内だけを抽出
    let inside: Vec<&Path> = given_folders
        .iter()
        .map(|p| p.as_path())
        .filter(|p| p.starts_with(&repo_root))
        .collect();
    println!("inside:{:?}", inside);
    let outside: Vec<&Path> = given_folders
        .iter()
        .map(|p| p.as_path())
        .filter(|p| !p.starts_with(&repo_root))
        .collect();

    // 3. 共通の親フォルダを探す
    let common = common_parent(&repo_root, &inside);

    // 4. ツリー作成
    let all_selected: HashSet<PathBuf> = given_folders.iter().cloned().collect();
    println!("all_selected:{:?}", all_selected);
    let mut nodes = vec![];

    if !inside.is_empty() {
        let repo_tree = build_tree(&common, &repo_root, &all_selected);
        nodes.push(repo_tree);
    }
    println!("nodes:{:?}", nodes);

    // 追加: リポジトリ外ノード
    for p in outside {
        nodes.push(build_external_node(p, &all_selected));
    }

    // 5. JSONでPrimeVueに渡す（仮想ルートノード）
    // println!("{}", serde_json::to_string_pretty(&nodes).unwrap());
    Ok(nodes)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_files,
            get_tree,
            sort_groups,
            add_file_to_groups,
            delete_file_nodes,
            get_include_tree_nodes,
            save_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
