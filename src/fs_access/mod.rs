use miette::Result;
use std::path::Path;

mod buffered;
pub use buffered::BufferedFsAccess;

pub trait FsAccess {
    /// Write all bytes to dst
    fn write_all(&mut self, dst: &Path, buf: &[u8]) -> Result<()>;

    /// Copy src to dst
    fn copy(&mut self, src: &Path, dst: &Path) -> Result<()>;

    /// Persist the changes if necessary
    fn persist(&mut self) -> Result<()>;
}
