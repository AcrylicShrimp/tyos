use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn entry_point(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;

    let expanded = quote! {
        #[unsafe(no_mangle)]
        #[unsafe(link_section = ".text.entry")]
        pub extern "C" fn _start() -> ! {
            let user_main: fn() -> isize = #fn_name;
            let exit_code = user_main();
            tyos_ulib::exit(exit_code);
        }

        #input_fn
    };

    TokenStream::from(expanded)
}
