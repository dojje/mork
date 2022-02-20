use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(message))]
struct Opts {
    msg_code: u8,
}

#[proc_macro_derive(Message, attributes(message))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, .. } = input;

    let msg_code = opts.msg_code;
    
    let to_raw = quote! {
        fn to_raw(&self) -> Vec<u8> {
            let mut og = bincode::serialize(&self).unwrap(); 

            let mut have_file = vec![#msg_code];
            have_file.append(&mut og);
            
            have_file

        }
    };

    let from_raw = quote! {
        fn from_raw(slice: &[u8]) -> Result<Self, &'static str> {
            if slice[0] != #msg_code {
                return Err("not good msg");
            }

            let have_file: Self = match bincode::deserialize(&slice[1..]) {
                Ok(v) => v,
                Err(_) => return Err("deserialising failed"),
            };

            Ok(have_file)
        }
    };

    let output = quote! {
        impl Message for #ident {
            #to_raw
            #from_raw
        }
    };

    output.into()
}
