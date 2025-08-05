mod init;
mod hash_object;
mod cat_file; 

pub use init::invoke as init;
pub use hash_object::invoke as hash_object;  
pub use cat_file::invoke as cat_file;   