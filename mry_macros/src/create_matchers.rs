use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Index};

use crate::alphabets::alphabets;

pub(crate) fn create() -> TokenStream {
    let items = alphabets(0..6).map(|args| {
        let (args, types): (Vec<_>, Vec<_>) = args
            .iter()
            .map(|name| {
                (
                    Ident::new(&name.to_lowercase(), Span::call_site()),
                    Ident::new(name, Span::call_site()),
                )
            })
            .unzip();
        let matchers: Vec<_> = types.iter().map(|ty| quote![ArgMatcher<#ty>]).collect();
        let trait_bounds: Vec<_> = types.iter().map(|ty| quote![#ty: Send + 'static]).collect();
        let matchers = quote![#(#matchers,)*];
        let matches = args.iter().enumerate().map(|(index, arg)| {
            let index = Index::from(index);
            quote![self.#index.matches(#arg)]
        });
        let args = quote![#(#args,)*];
        quote! {
            impl<#(#trait_bounds),*> Match<(#(#types,)*)> for (#matchers) {
                fn matches(&self, (#args): &(#(#types,)*)) -> bool {
                    #(#matches &&)* true
                }
            }

            impl<#(#trait_bounds),*> From<(#matchers)> for Matcher<(#(#types,)*)> {
                fn from((#args): (#matchers)) -> Self {
                    Matcher(Box::new((#args)))
                }
            }
        }
    });
    quote![#(#items)*]
}
