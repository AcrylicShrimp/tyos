use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn entry_point(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let expanded = match input_fn.sig.inputs.len() {
        0 => {
            quote! {
                #[unsafe(no_mangle)]
                #[unsafe(link_section = ".text.entry")]
                pub extern "C" fn _start(_arg0: usize) -> ! {
                    let user_main: fn() -> isize = #fn_name;
                    let exit_code = user_main();
                    tyos_ulib::exit(exit_code);
                }

                #input_fn
            }
        }
        1 => {
            quote! {
                #[unsafe(no_mangle)]
                #[unsafe(link_section = ".text.entry")]
                pub extern "C" fn _start(arg0: usize) -> ! {
                    let user_main: fn(usize) -> isize = #fn_name;
                    let exit_code = user_main(arg0);
                    tyos_ulib::exit(exit_code);
                }

                #input_fn
            }
        }
        _ => {
            return syn::Error::new_spanned(
                &input_fn.sig,
                "main must take either no arguments or one usize argument",
            )
            .to_compile_error()
            .into();
        }
    };

    TokenStream::from(expanded)
}
