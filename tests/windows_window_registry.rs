#[macro_export]
macro_rules! log_important {
    ($level:ident, $($arg:tt)*) => {{
        let _ = stringify!($level);
        let _ = format_args!($($arg)*);
    }};
}

#[path = "../src/rust/ui/window_registry.rs"]
mod window_registry;
