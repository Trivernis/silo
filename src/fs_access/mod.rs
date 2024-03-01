use miette::Result;
use std::{fs::Permissions, path::Path};

mod buffered;
pub use buffered::BufferedFsAccess;

pub trait FsAccess {
    /// Write all bytes to dst
    fn write_all(&mut self, dst: &Path, buf: &[u8]) -> Result<()>;

    /// Copy src to dst
    fn copy(&mut self, src: &Path, dst: &Path) -> Result<()>;

    /// Sets permissions on a file
    fn set_permissions(&mut self, path: &Path, perm: Permissions) -> Result<()>;

    /// Persist the changes if necessary
    fn persist(&mut self) -> Result<()>;
}
