use std::io::ErrorKind;
use std::path::Path;

pub enum FileType {
    FILE,
    DIR,
    SYMLINK,
}

pub async fn exist<P: AsRef<Path>>(path: P, ty: FileType) -> std::io::Result<bool> {
    let result = tokio::fs::metadata(path).await;
    let meta = match result {
        Ok(o) => o,
        Err(err) => {
            return match err.kind() {
                ErrorKind::NotFound => Ok(false),
                _ => Err(err),
            }
        }
    };
    Ok(match ty {
        FileType::FILE => meta.is_file(),
        FileType::DIR => meta.is_dir(),
        FileType::SYMLINK => meta.is_symlink(),
    })
}
