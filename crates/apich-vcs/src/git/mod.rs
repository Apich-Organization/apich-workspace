pub mod bridge;
pub mod credentials;
pub mod lfs;
pub mod material;

pub use bridge::GitBridge;
pub use credentials::{GitAuth, GitRemoteConfig};
pub use lfs::LfsPolicy;
pub use material::{MaterialManager, MaterialRecord};
