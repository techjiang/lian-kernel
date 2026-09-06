//! Virtual File System (VFS) — a minimal, trait-based filesystem layer.
//!
//! The VFS provides a unified path-based interface that can front multiple
//! filesystem backends (ramfs, devfs, etc.).

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use crate::sync::spinlock::SpinLock;

/// VFS node type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
    /// Character device (e.g. /dev/null, /dev/tty).
    CharDev,
}

/// A VFS node.
pub struct VfsNode {
    pub name: String,
    pub node_type: NodeType,
    pub data: Vec<u8>,
    pub children: BTreeMap<String, VfsNode>,
}

impl VfsNode {
    pub fn new_dir(name: &str) -> Self {
        VfsNode {
            name: name.to_string(),
            node_type: NodeType::Directory,
            data: Vec::new(),
            children: BTreeMap::new(),
        }
    }

    pub fn new_file(name: &str) -> Self {
        VfsNode {
            name: name.to_string(),
            node_type: NodeType::File,
            data: Vec::new(),
            children: BTreeMap::new(),
        }
    }

    pub fn new_chardev(name: &str) -> Self {
        VfsNode {
            name: name.to_string(),
            node_type: NodeType::CharDev,
            data: Vec::new(),
            children: BTreeMap::new(),
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        self.data.clear();
        self.data.extend_from_slice(data);
    }

    pub fn append(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    pub fn read(&self, offset: usize, len: usize) -> &[u8] {
        let end = (offset + len).min(self.data.len());
        if offset >= self.data.len() {
            return &[];
        }
        &self.data[offset..end]
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// The global VFS root.  Initialized in `init()`.
static ROOT: SpinLock<Option<VfsNode>> = SpinLock::new(None);

/// Initialize the VFS with a basic directory structure.
pub fn init() {
    let mut root = VfsNode::new_dir("/");

    // Create /dev directory.
    let mut dev = VfsNode::new_dir("dev");
    dev.children.insert("null".to_string(), VfsNode::new_chardev("null"));
    dev.children.insert("tty".to_string(), VfsNode::new_chardev("tty"));
    dev.children.insert("ttyS0".to_string(), VfsNode::new_chardev("ttyS0"));
    root.children.insert("dev".to_string(), dev);

    // Create /proc directory (placeholder).
    let proc = VfsNode::new_dir("proc");
    root.children.insert("proc".to_string(), proc);

    // Create /sys directory (placeholder).
    let sys = VfsNode::new_dir("sys");
    root.children.insert("sys".to_string(), sys);

    // Create /bin directory (for userspace binaries).
    let bin = VfsNode::new_dir("bin");
    root.children.insert("bin".to_string(), bin);

    // Create a welcome file.
    let mut welcome = VfsNode::new_file("welcome.txt");
    welcome.write(b"Welcome to Lian OS!\n");
    root.children.insert("welcome.txt".to_string(), welcome);

    crate::println!("[vfs] VFS initialized with /dev /proc /sys /bin");

    // Store the root.
    *ROOT.lock() = Some(root);
}

/// Resolve a path (e.g. "/dev/tty") to a mutable node reference.
///
/// Returns `Err` if the path is not found.
fn resolve<'a>(root: &'a mut VfsNode, path: &str) -> Result<&'a mut VfsNode, &'static str> {
    let path = path.strip_prefix('/').unwrap_or(path);
    if path.is_empty() {
        return Ok(root);
    }

    let mut node = root;
    for component in path.split('/') {
        match node.children.get_mut(component) {
            Some(child) => node = child,
            None => return Err("path not found"),
        }
    }
    Ok(node)
}

/// Read a file's contents by path.
pub fn read_file(path: &str) -> Option<Vec<u8>> {
    let mut guard = ROOT.lock();
    let root = guard.as_mut()?;
    match resolve(root, path) {
        Ok(node) if node.node_type == NodeType::File => Some(node.data.clone()),
        _ => None,
    }
}

/// Write data to a file (creates it if it doesn't exist).
pub fn write_file(path: &str, data: &[u8]) -> Result<(), &'static str> {
    let mut guard = ROOT.lock();
    let root = guard.as_mut().ok_or("VFS not initialized")?;
    // Split path into parent dir and filename.
    let (dir_path, file_name) = match path.rfind('/') {
        Some(idx) => (&path[..idx], &path[idx + 1..]),
        None => ("", path),
    };

    let dir = resolve(root, dir_path)?;
    if dir.node_type != NodeType::Directory {
        return Err("parent is not a directory");
    }

    let file = dir.children.entry(file_name.to_string()).or_insert_with(|| VfsNode::new_file(file_name));
    file.write(data);
    Ok(())
}

/// List the children of a directory by path.
pub fn list_dir(path: &str) -> Option<Vec<String>> {
    let mut guard = ROOT.lock();
    let root = guard.as_mut()?;
    match resolve(root, path) {
        Ok(node) if node.node_type == NodeType::Directory => {
            Some(node.children.keys().cloned().collect())
        }
        _ => None,
    }
}

/// Dump the entire VFS tree (for debugging).
pub fn dump() {
    let guard = ROOT.lock();
    let root = match guard.as_ref() {
        Some(r) => r,
        None => {
            crate::println!("[vfs] VFS not initialized");
            return;
        }
    };
    fn print_node(node: &VfsNode, indent: usize) {
        let prefix: String = " ".repeat(indent);
        let type_str = match node.node_type {
            NodeType::Directory => "[DIR] ",
            NodeType::File => "[FILE]",
            NodeType::CharDev => "[DEV] ",
        };
        crate::println!("{}{} {} ({} bytes)", prefix, type_str, node.name, node.size());
        for child in node.children.values() {
            print_node(child, indent + 2);
        }
    }
    crate::println!("--- VFS Tree ---");
    print_node(root, 0);
    crate::println!("--- End ---");
}
