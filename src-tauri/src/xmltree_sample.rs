use std::collections::HashMap;
use std::fs;
use xmltree::{Element, XMLNode};

/// キャッシュ構造体
#[derive(Debug)]
struct ProjectCache {
    file_path: String,
    project: Element,                 // projectノード全体
    target: Element,                  // targetノード全体
    groups: HashMap<String, Element>, // group_name -> groupノード
}

impl ProjectCache {
    /// XML ファイルを読み込んでキャッシュにする
    fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let xml_str = fs::read_to_string(path)?;
        let root: Element = xmltree::Element::parse(xml_str.as_bytes())?;

        // project ノード
        let project = root.clone();

        // target ノード (project直下に1つ想定)
        let target = project
            .get_child("target")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("target not found"))?;

        // groups (target直下に複数想定)
        let mut groups = HashMap::new();
        for group in target.children.iter().filter_map(|n| {
            if let XMLNode::Element(e) = n {
                if e.name == "group" {
                    Some(e.clone())
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            if let Some(name) = group.attributes.get("name") {
                groups.insert(name.clone(), group);
            }
        }

        Ok(Self {
            file_path: path.to_string(),
            project,
            target,
            groups,
        })
    }

    /// group の file を name 属性でソート
    fn sort_group_files(&mut self, group_name: &str) {
        if let Some(group) = self.groups.get_mut(group_name) {
            // file ノードのみ取り出す
            let mut files: Vec<Element> = group
                .children
                .iter()
                .filter_map(|n| {
                    if let XMLNode::Element(e) = n {
                        if e.name == "file" {
                            return Some(e.clone());
                        }
                    }
                    None
                })
                .collect();

            // ソート
            files.sort_by_key(|f| f.attributes.get("name").cloned().unwrap_or_default());

            // group.children を再構築
            group.children.retain(|n| {
                if let XMLNode::Element(e) = n {
                    e.name != "file"
                } else {
                    true
                }
            });
            for f in files {
                group.children.push(XMLNode::Element(f));
            }
        }
    }

    /// file を削除
    fn remove_file(&mut self, group_name: &str, file_name: &str) {
        if let Some(group) = self.groups.get_mut(group_name) {
            group.children.retain(|n| {
                if let XMLNode::Element(e) = n {
                    !(e.name == "file"
                        && e.attributes
                            .get("name")
                            .map(|s| s == file_name)
                            .unwrap_or(false))
                } else {
                    true
                }
            });
        }
    }

    /// group に file を追加
    fn add_file(&mut self, group_name: &str, file_name: &str) {
        if let Some(group) = self.groups.get_mut(group_name) {
            let mut file_elem = Element::new("file");
            file_elem
                .attributes
                .insert("name".to_string(), file_name.to_string());
            group.children.push(XMLNode::Element(file_elem));
        }
    }

    /// 保存（上書き）
    fn save(&self) -> anyhow::Result<()> {
        let mut buf = Vec::new();
        self.project.write(&mut buf)?;
        fs::write(&self.file_path, buf)?;
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    // 複数ファイルをロード
    let mut caches = Vec::new();
    for path in ["sample1.xml", "sample2.xml"] {
        let cache = ProjectCache::load_from_file(path)?;
        caches.push(cache);
    }

    // 操作例
    let cache = caches.get_mut(0).unwrap();
    cache.sort_group_files("group1");
    cache.remove_file("group1", "fileA.c");
    cache.add_file("group1", "new_file.c");

    // 保存
    cache.save()?;

    Ok(())
}
