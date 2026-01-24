extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Block, Ident, ItemFn, Lit, Signature, Token, parse::Parse, parse_macro_input};

// 过程宏可选参数
#[derive(Debug, Default)]
struct EntryAttr {
    log_level: Option<String>,
}

impl EntryAttr {
    fn get_log_level(&self) -> &str {
        self.log_level.as_deref().unwrap_or("info")
    }
}

// 过程宏参数解析
impl Parse for EntryAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut attr = EntryAttr::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: Lit = input.parse()?;
            match key.to_string().as_ref() {
                "log_level" => attr.log_level.replace(match value {
                    Lit::Str(s) => s.value(),
                    _ => {
                        return Err(syn::Error::new_spanned(
                            value,
                            "log_level field must be a string",
                        ));
                    }
                }),
                other => {
                    return Err(syn::Error::new_spanned(
                        key,
                        format!("unsupport key: {}", other),
                    ));
                }
            };
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(attr)
    }
}

#[proc_macro_attribute]
pub fn main(args: TokenStream, item: TokenStream) -> TokenStream {
    let func: ItemFn = parse_macro_input!(item);
    if func.sig.asyncness.is_none() {
        return error_stream(&func.sig, "should be an async function");
    }
    if func.sig.ident == "main" && !func.sig.inputs.is_empty() {
        return error_stream(&func.sig, "main() function should without arguments");
    }

    let body = &func.block;
    let attr = parse_macro_input!(args as EntryAttr);

    let import = import_stream(false);
    let logger_init = logger_init_stream(&attr);
    let body = body_stream(body);
    let expanded = quote! {
        fn main() {
            #import

            #logger_init
            #body
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
    let func: ItemFn = parse_macro_input!(item);
    if func.sig.asyncness.is_none() {
        return error_stream(&func.sig, "should be an async function");
    }
    let fn_name = func.sig.ident;
    let fn_name_str = fn_name.to_string();
    let body = &func.block;

    let attr = parse_macro_input!(args as EntryAttr);

    let import = import_stream(true);
    let logger_init = logger_init_stream(&attr);
    let body = body_stream(body);
    let expanded = quote! {
        #[test]
        fn #fn_name() {
            #import

            #logger_init
            log::debug!("run test <{}>", #fn_name_str);
            #body
        }
    };

    TokenStream::from(expanded)
}

fn error_stream(sig: &Signature, msg: &str) -> TokenStream {
    syn::Error::new_spanned(sig, msg)
        .into_compile_error()
        .into()
}

fn import_stream(is_crate: bool) -> proc_macro2::TokenStream {
    if is_crate {
        quote! {
            use crate::{init_logger, run, spawn};
        }
    } else {
        quote! {
            use mini_runtime::{init_logger, run, spawn};
        }
    }
}

fn logger_init_stream(entry_attr: &EntryAttr) -> proc_macro2::TokenStream {
    let log_level_str = entry_attr.get_log_level();
    quote! {
        let log_level = match #log_level_str {
            "error" => log::LevelFilter::Error,
            "warn" => log::LevelFilter::Warn,
            "info" => log::LevelFilter::Info,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            _ => {
                eprintln!("unsupport log level '{}', use level 'info' default", #log_level_str);
                log::LevelFilter::Info
            }
        };
        init_logger(log_level);
    }
}

fn body_stream(body: &Block) -> proc_macro2::TokenStream {
    quote! {
        spawn!(async { #body }, |result| {
            log::info!("runtime output: {:?}", result);
        });

        run();
    }
}
