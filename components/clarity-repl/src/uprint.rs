// universal print macro

#[macro_export]
#[cfg(not(target_arch = "wasm32"))]
macro_rules! uprint {
    ( $( $t:tt )* ) => {
        println!($( $t )* );
    }
}

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! uprint {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}
