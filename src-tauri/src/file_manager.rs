//! File manager: browse / read / write / manage an instance's `core/` directory.
//!
//! Ported (and adapted) from MSLX-1.5.3's file manager. In AstrBot's layout each
//! instance stores its AstrBot runtime under `<instance_dir>/core`, which plays the
//! same role as MSLX's `server.Base` — the single jail root for all file ops.
//! Every path is forced through [`safe_join`] to prevent path traversal, mirroring
//! MSLX's `FileUtils.GetSafePath`.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use axum::body::Bytes;
use axum::extract::{Path as AxumPath, Query};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use tokio::fs as async_fs;
use walkdir::WalkDir;

use crate::error::{AppError, Result};
use crate::utils::paths::get_instance_core_dir;
use crate::utils::validation::validate_instance_id;

// ---------------------------------------------------------------------------
// path safety
// ---------------------------------------------------------------------------

/// Normalize a path without touching the filesystem: resolve `.` and `..`,
/// drop absolute/root markers that would escape the base.
fn normalize(p: PathBuf) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s),
            Component::RootDir => out.push("/"),
            Component::Prefix(p) => out.push(p.as_os_str()),
        }
    }
    out
}

/// Join `root` with a user-supplied relative `rel`, refusing anything that
/// escapes `root`. Returns the absolute, normalized path.
fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel = rel.trim_start_matches('/');
    let joined = root.join(rel);
    let norm = normalize(joined);
    let norm_root = normalize(root.to_path_buf());
    let root_prefix = format!("{}/", norm_root.display());
    let escaped = norm != norm_root && !norm.to_string_lossy().starts_with(&root_prefix);
    if escaped {
        return Err(AppError::other("路径越界（禁止访问 core 目录之外）"));
    }
    Ok(norm)
}

/// Resolve instance id -> existing core root directory.
fn resolve_root(id: &str) -> Result<PathBuf> {
    validate_instance_id(id)?;
    let root = get_instance_core_dir(id);
    if !root.exists() {
        return Err(AppError::other("实例 core 目录不存在"));
    }
    Ok(root)
}

fn ioerr(e: std::io::Error) -> AppError {
    AppError::io(e.to_string())
}

fn json_err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "error": msg.into() }))).into_response()
}

// ---------------------------------------------------------------------------
// listing & content
// ---------------------------------------------------------------------------

pub async fn file_lists(
    AxumPath(id): AxumPath<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let rel = q.get("path").map(String::as_str).unwrap_or("");
    let p = match safe_join(&root, rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !p.is_dir() {
        return json_err(StatusCode::BAD_REQUEST, "不是目录");
    }
    match list_dir(&p).await {
        Ok(items) => Json(json!({ "path": rel, "items": items })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

async fn list_dir(p: &Path) -> Result<Value> {
    let mut rd = async_fs::read_dir(p).await.map_err(ioerr)?;
    let mut items: Vec<Value> = Vec::new();
    while let Some(entry) = rd.next_entry().await.map_err(ioerr)? {
        let meta = entry.metadata().await.map_err(ioerr)?;
        let is_dir = meta.is_dir();
        let size = if is_dir { 0u64 } else { meta.len() };
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        items.push(json!({
            "name": entry.file_name().to_string_lossy().to_string(),
            "is_dir": is_dir,
            "size": size,
            "modified": modified,
        }));
    }
    items.sort_by(|a, b| {
        let ad = a["is_dir"].as_bool().unwrap_or(false);
        let bd = b["is_dir"].as_bool().unwrap_or(false);
        if ad != bd {
            return bd.cmp(&ad); // directories first
        }
        a["name"].as_str().cmp(&b["name"].as_str())
    });
    Ok(Value::Array(items))
}

pub async fn file_content_get(
    AxumPath(id): AxumPath<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let rel = match q.get("path") {
        Some(p) => p.as_str(),
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 path"),
    };
    let p = match safe_join(&root, rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !p.is_file() {
        return json_err(StatusCode::BAD_REQUEST, "不是文件");
    }
    match async_fs::read_to_string(&p).await {
        Ok(content) => Json(json!({ "path": rel, "content": content })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn file_content_post(
    AxumPath(id): AxumPath<String>,
    Json(body): Json<Value>,
) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let rel = match body.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 path"),
    };
    let content = match body.get("content").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 content"),
    };
    let p = match safe_join(&root, rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    // writing a file must stay inside an existing parent directory
    if let Some(parent) = p.parent() {
        if !parent.exists() {
            return json_err(StatusCode::BAD_REQUEST, "父目录不存在");
        }
    }
    match async_fs::write(&p, content).await {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// create / rename / delete / copy / move
// ---------------------------------------------------------------------------

pub async fn file_mkdir(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let rel = match body.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 path"),
    };
    let p = match safe_join(&root, rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if p.exists() {
        return json_err(StatusCode::BAD_REQUEST, "目标已存在");
    }
    match async_fs::create_dir_all(&p).await {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn file_rename(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let old = match body.get("old_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 old_path"),
    };
    let new = match body.get("new_path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 new_path"),
    };
    let src = match safe_join(&root, old) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let dst = match safe_join(&root, new) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !src.exists() {
        return json_err(StatusCode::BAD_REQUEST, "源不存在");
    }
    if let Some(parent) = dst.parent() {
        if !parent.exists() {
            let _ = async_fs::create_dir_all(parent).await;
        }
    }
    match async_fs::rename(&src, &dst).await {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn file_delete(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let paths = match body.get("paths").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 paths"),
    };
    let mut removed = Vec::new();
    for v in paths {
        let rel = match v.as_str() {
            Some(s) => s,
            None => continue,
        };
        let p = match safe_join(&root, rel) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let res = if p.is_dir() {
            async_fs::remove_dir_all(&p).await
        } else {
            async_fs::remove_file(&p).await
        };
        if res.is_ok() {
            removed.push(rel.to_string());
        }
    }
    Json(json!({ "ok": true, "removed": removed })).into_response()
}

/// Copy `sources` (relative) into `dest_dir` (relative). Files overwrite.
pub async fn file_copy(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let sources = match body.get("sources").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 sources"),
    };
    let dest_rel = match body.get("dest_dir").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 dest_dir"),
    };
    let dest_dir = match safe_join(&root, dest_rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !dest_dir.exists() {
        let _ = async_fs::create_dir_all(&dest_dir).await;
    }
    let mut copied = Vec::new();
    for v in sources {
        let rel = match v.as_str() {
            Some(s) => s,
            None => continue,
        };
        let src = match safe_join(&root, rel) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if src.is_dir() {
            if copy_dir_recursive(&src, &dest_dir.join(src.file_name().unwrap())).await.is_ok() {
                copied.push(rel.to_string());
            }
        } else if copy_file(&src, &dest_dir.join(src.file_name().unwrap()))
            .await
            .is_ok()
        {
            copied.push(rel.to_string());
        }
    }
    Json(json!({ "ok": true, "copied": copied })).into_response()
}

/// Move `sources` (relative) into `dest_dir` (relative).
pub async fn file_move(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let sources = match body.get("sources").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 sources"),
    };
    let dest_rel = match body.get("dest_dir").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 dest_dir"),
    };
    let dest_dir = match safe_join(&root, dest_rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !dest_dir.exists() {
        let _ = async_fs::create_dir_all(&dest_dir).await;
    }
    let mut moved = Vec::new();
    for v in sources {
        let rel = match v.as_str() {
            Some(s) => s,
            None => continue,
        };
        let src = match safe_join(&root, rel) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let dst = dest_dir.join(src.file_name().unwrap());
        if async_fs::rename(&src, &dst).await.is_ok() {
            moved.push(rel.to_string());
        }
    }
    Json(json!({ "ok": true, "moved": moved })).into_response()
}

async fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        async_fs::create_dir_all(parent).await.map_err(ioerr)?;
    }
    async_fs::copy(src, dst).await.map_err(ioerr)?;
    Ok(())
}

async fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    async_fs::create_dir_all(dst).await.map_err(ioerr)?;
    let mut rd = async_fs::read_dir(src).await.map_err(ioerr)?;
    while let Some(entry) = rd.next_entry().await.map_err(ioerr)? {
        let path = entry.path();
        let meta = entry.metadata().await.map_err(ioerr)?;
        let target = dst.join(entry.file_name());
        if meta.is_dir() {
            Box::pin(copy_dir_recursive(&path, &target)).await?;
        } else {
            copy_file(&path, &target).await?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// chmod (unix only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
pub async fn file_chmod(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    use std::os::unix::fs::PermissionsExt;
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let rel = match body.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 path"),
    };
    let mode = match body.get("mode").and_then(|v| v.as_u64()) {
        Some(m) => m as u32,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 mode"),
    };
    let p = match safe_join(&root, rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !p.exists() {
        return json_err(StatusCode::BAD_REQUEST, "目标不存在");
    }
    let perm = std::fs::Permissions::from_mode(mode);
    match std::fs::set_permissions(&p, perm) {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

#[cfg(not(unix))]
pub async fn file_chmod(AxumPath(_): AxumPath<String>, _: Json<Value>) -> Response {
    json_err(StatusCode::BAD_REQUEST, "chmod 仅在类 Unix 系统可用")
}

// ---------------------------------------------------------------------------
// download (whole-file read; core files are small config/source)
// ---------------------------------------------------------------------------

pub async fn file_download(
    AxumPath(id): AxumPath<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let rel = match q.get("path") {
        Some(p) => p.as_str(),
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 path"),
    };
    let p = match safe_join(&root, rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !p.is_file() {
        return json_err(StatusCode::BAD_REQUEST, "不是文件");
    }
    let data = match async_fs::read(&p).await {
        Ok(d) => d,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let fname = p.file_name().unwrap().to_string_lossy().to_string();
    let cd = format!("attachment; filename=\"{}\"", fname);
    (
        [
            (header::CONTENT_DISPOSITION, cd),
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
        ],
        Bytes::from(data),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// chunked upload (raw bytes per chunk, sequential append)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct UploadSession {
    dest: PathBuf,
    tmp: PathBuf,
}

static UPLOADS: Lazy<Mutex<HashMap<String, UploadSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn upload_init(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let dir = body.get("dir").and_then(|v| v.as_str()).unwrap_or("");
    let name = match body.get("name").and_then(|v| v.as_str()) {
        Some(n) => n,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 name"),
    };
    let parent = match safe_join(&root, dir) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !parent.is_dir() {
        return json_err(StatusCode::BAD_REQUEST, "目标目录不存在");
    }
    let dest = parent.join(name);
    let upload_id = uuid::Uuid::new_v4().to_string();
    let tmp = std::env::temp_dir().join(format!("astrbot_upload_{}", upload_id));
    {
        let mut map = UPLOADS.lock().unwrap();
        map.insert(
            upload_id.clone(),
            UploadSession {
                dest: dest.clone(),
                tmp: tmp.clone(),
            },
        );
    }
    // start empty temp file
    if let Err(e) = std::fs::File::create(&tmp) {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    Json(json!({ "upload_id": upload_id })).into_response()
}

pub async fn upload_chunk(
    AxumPath(id): AxumPath<String>,
    AxumPath(upload_id): AxumPath<String>,
    bytes: Bytes,
) -> Response {
    if validate_instance_id(&id).is_err() {
        return json_err(StatusCode::BAD_REQUEST, "invalid instance id");
    }
    let session = {
        let map = UPLOADS.lock().unwrap();
        match map.get(&upload_id) {
            Some(s) => s.clone(),
            None => return json_err(StatusCode::BAD_REQUEST, "无效 upload_id"),
        }
    };
    // sequential append
    let mut f = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&session.tmp)
    {
        Ok(f) => f,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    if let Err(e) = f.write_all(&bytes) {
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    Json(json!({ "ok": true })).into_response()
}

pub async fn upload_finish(
    AxumPath(id): AxumPath<String>,
    AxumPath(upload_id): AxumPath<String>,
) -> Response {
    if validate_instance_id(&id).is_err() {
        return json_err(StatusCode::BAD_REQUEST, "invalid instance id");
    }
    let session = {
        let mut map = UPLOADS.lock().unwrap();
        match map.remove(&upload_id) {
            Some(s) => s,
            None => return json_err(StatusCode::BAD_REQUEST, "无效 upload_id"),
        }
    };
    if let Some(parent) = session.dest.parent() {
        if !parent.exists() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let res = std::fs::rename(&session.tmp, &session.dest);
    if let Err(e) = res {
        // fallback copy
        let _ = std::fs::copy(&session.tmp, &session.dest);
        let _ = std::fs::remove_file(&session.tmp);
        return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    Json(json!({ "ok": true, "path": session.dest.to_string_lossy().to_string() })).into_response()
}

// ---------------------------------------------------------------------------
// compress / decompress (zip)
// ---------------------------------------------------------------------------

pub async fn file_compress(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let sources = match body.get("sources").and_then(|v| v.as_array()) {
        Some(a) => a.clone(),
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 sources"),
    };
    let dest_rel = match body.get("dest").and_then(|v| v.as_str()) {
        Some(d) => d,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 dest"),
    };
    let dest = match safe_join(&root, dest_rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if let Some(parent) = dest.parent() {
        if !parent.exists() {
            let _ = async_fs::create_dir_all(parent).await;
        }
    }
    // collect files first (path traversal already checked by safe_join per source)
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for v in &sources {
        let rel = match v.as_str() {
            Some(s) => s,
            None => continue,
        };
        let src = match safe_join(&root, rel) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if !src.exists() {
            continue;
        }
        if src.is_dir() {
            for entry in WalkDir::new(&src).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let abs = entry.path().to_path_buf();
                    let rel_name = abs
                        .strip_prefix(&src)
                        .unwrap_or_else(|_| entry.path())
                        .to_string_lossy()
                        .to_string();
                    let arc_name = format!(
                        "{}/{}",
                        src.file_name().unwrap().to_string_lossy(),
                        rel_name
                    );
                    files.push((abs, arc_name));
                }
            }
        } else {
            let arc_name = src.file_name().unwrap().to_string_lossy().to_string();
            files.push((src, arc_name));
        }
    }
    match write_zip(&dest, &files) {
        Ok(_) => Json(json!({ "ok": true, "dest": dest_rel })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

fn write_zip(dest: &Path, files: &[(PathBuf, String)]) -> std::io::Result<()> {
    let file = std::fs::File::create(dest)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (abs, arc_name) in files {
        let data = std::fs::read(abs)?;
        zip.start_file(arc_name, opts)?;
        zip.write_all(&data)?;
    }
    zip.finish()?;
    Ok(())
}

pub async fn file_decompress(AxumPath(id): AxumPath<String>, Json(body): Json<Value>) -> Response {
    let root = match resolve_root(&id) {
        Ok(r) => r,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let src_rel = match body.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return json_err(StatusCode::BAD_REQUEST, "缺少 path"),
    };
    let dest_rel = body.get("dest_dir").and_then(|v| v.as_str()).unwrap_or("");
    let src = match safe_join(&root, src_rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    let dest_dir = match safe_join(&root, dest_rel) {
        Ok(p) => p,
        Err(e) => return json_err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    if !src.is_file() {
        return json_err(StatusCode::BAD_REQUEST, "压缩包不存在");
    }
    if !dest_dir.exists() {
        let _ = async_fs::create_dir_all(&dest_dir).await;
    }
    match extract_zip(&src, &dest_dir) {
        Ok(n) => Json(json!({ "ok": true, "count": n })).into_response(),
        Err(e) => json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

fn extract_zip(src: &Path, dest_dir: &Path) -> std::io::Result<usize> {
    let file = std::fs::File::open(src)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut count = 0;
    for i in 0..archive.len() {
        let mut zf = archive.by_index(i)?;
        // guard against zip-slip
        let name = zf.name().to_string();
        let out = match safe_join(dest_dir, &name) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if zf.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut buf = Vec::new();
            zf.read_to_end(&mut buf)?;
            std::fs::write(&out, &buf)?;
            count += 1;
        }
    }
    Ok(count)
}
