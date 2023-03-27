use std::path::Path;

pub enum FileType{
    FILE,
    DIR,
    SYMLINK,
}

pub async fn exist<P:AsRef<Path>>(path:P,ty:FileType)->std::io::Result<bool>{
    let meta = tokio::fs::metadata(path).await?;
    Ok(match ty {
        FileType::FILE => meta.is_file(),
        FileType::DIR => meta.is_dir(),
        FileType::SYMLINK => meta.is_symlink(),
    })
}