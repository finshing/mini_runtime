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

    let use_stream = import(false);
    let init_logger_stream = init_logger(&attr);
    let body_stream = gen_body(body);
    let expanded = quote! {
        fn main() {
            #use_stream

            #init_logger_stream
            #body_stream
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

    let use_stream = import(true);
    let init_logger_stream = init_logger(&attr);
    let body_stream = gen_body(body);
    let expanded = quote! {
        #[test]
        fn #fn_name() {
            #use_stream

            #init_logger_stream
            log::debug!("run test <{}>", #fn_name_str);
            #body_stream
        }
    };

    TokenStream::from(expanded)
}

fn error_stream(sig: &Signature, msg: &str) -> TokenStream {
    syn::Error::new_spanned(sig, msg)
        .into_compile_error()
        .into()
}

fn import(is_crate: bool) -> proc_macro2::TokenStream {
    if is_crate {
        quote! {
            use crate::{init_logger, spawn, run};
        }
    } else {
        quote! {
            use mini_runtime::{init_logger, spawn, run};
        }
    }
}

fn init_logger(entry_attr: &EntryAttr) -> proc_macro2::TokenStream {
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

fn gen_body(body: &Block) -> proc_macro2::TokenStream {
    quote! {
        let body = async #body;
        spawn(body);
        run();
    }
}
