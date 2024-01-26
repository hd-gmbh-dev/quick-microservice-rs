#[macro_export]
macro_rules! include_roles {
    ($filename:tt) => {
        include!(concat!(env!("OUT_DIR"), "/", $filename,".rs"));
    };
}