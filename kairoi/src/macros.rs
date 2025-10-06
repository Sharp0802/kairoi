
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:expr),+) => {
        $crate::Event::log($level, format!($($arg),+)).submit();
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:expr),+) => {
        $crate::log!($crate::Level::Error, $($arg),+);
    }
}

#[macro_export]
macro_rules! warn {
    ($($arg:expr),+) => {
        $crate::log!($crate::Level::Warn, $($arg),+);
    }
}

#[macro_export]
macro_rules! info {
    ($($arg:expr),+) => {
        $crate::log!($crate::Level::Info, $($arg),+);
    }
}

#[macro_export]
macro_rules! debug {
    ($($arg:expr),+) => {
        $crate::log!($crate::Level::Debug, $($arg),+);
    }
}

#[macro_export]
macro_rules! trace {
    ($($arg:expr),+) => {
        $crate::log!($crate::Level::Trace, $($arg),+);
    }
}
