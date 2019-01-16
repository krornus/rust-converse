use proc_macro2::TokenStream;
use quote::quote;
use syn;

type Arguments = syn::punctuated::Punctuated<syn::Pat, syn::token::Comma>;

pub struct StateImpl<'a> {
    item: &'a syn::ItemImpl,
}

impl<'a> StateImpl<'a> {
    pub fn new(item: &'a syn::ItemImpl) -> StateImpl {
        StateImpl {
            item: item,
        }
    }

    pub fn ty(&self) -> TokenStream {
        let ty = &self.item.self_ty;
        quote! { #ty }
    }

    pub fn generics(&self) -> TokenStream {
        let gen = &self.item.generics;
        quote!( #gen )
    }

    pub fn params(&self) -> Vec<TokenStream> {
        let mut lf = self.lifetimes();
        lf.extend_from_slice(&self.types());
        lf
    }


    pub fn lifetimes(&self) -> Vec<TokenStream> {
        self.item.generics.params.iter()
            .filter_map(|x| {
                match x {
                    syn::GenericParam::Lifetime(x) => Some(x),
                    _ => None,
                }
            })
            .map(|x| {
                let attrs: TokenStream = x.attrs.iter().map(|x| quote!{ #x }).collect();
                let lt = &x.lifetime;

                quote!{ #attrs #lt }
            }).collect()

    }

    pub fn types(&self) -> Vec<TokenStream> {
        self.item.generics.params.iter()
            .filter_map(|x| {
                match x {
                    syn::GenericParam::Type(x) => Some(x),
                    _ => None,
                }
            })
            .map(|x| {
                let attrs: TokenStream = x.attrs.iter().map(|x| quote!{ #x }).collect();
                let id = &x.ident;

                quote!{ #attrs #id }
            }).collect()
    }

    pub fn implement(&self) -> TokenStream {

        let attrs: TokenStream = self.item.attrs.iter().map(|x| quote!{ #x }).collect();
        let defaultness = &self.item.defaultness;
        let unsafety = &self.item.unsafety;
        let generics = &self.item.generics;
        let ty = self.ty();

        quote! { #attrs impl #defaultness #unsafety #generics #ty }

    }

    pub fn fabricate(&self, ty: TokenStream) -> TokenStream {

        let attrs: TokenStream = self.item.attrs.iter().map(|x| quote!{ #x }).collect();
        let defaultness = &self.item.defaultness;
        let unsafety = &self.item.unsafety;
        let generics = &self.item.generics;

        quote! { #attrs impl #defaultness #unsafety #generics #ty }
    }
}

pub struct StateMethod<'a> {
    item: &'a syn::ItemImpl,
    method: &'a syn::ImplItemMethod,
}

impl<'a> StateMethod<'a> {
    pub fn new(item: &'a syn::ItemImpl, method: &'a syn::ImplItemMethod) -> Self {
        StateMethod {
            item: item,
            method: method,
        }
    }

    pub fn is_static(&self) -> bool {
        if let Some(pair) = self.method.sig.decl.inputs.first() {
            match pair.value() {
                syn::FnArg::SelfRef(_) => false,
                syn::FnArg::SelfValue(_) => false,
                _ => true,
            }
        } else {
            true
        }
    }

    pub fn arguments(&self) -> Arguments {
        self.method.sig.decl.inputs.pairs()
            .filter(|x| match x.value() {
                syn::FnArg::SelfRef(_) => false,
                syn::FnArg::SelfValue(_) => false,
                _ => true,
            })
            .filter_map(|x| {
                match x.value() {
                    /* make a new pair without type attribution */
                    syn::FnArg::Captured(arg) => Some(syn::punctuated::Pair::new(
                        arg.pat.clone(),
                        x.punct().map(|x| (**x).clone())
                    )),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn ident(&self) -> &'a syn::Ident {
        &self.method.sig.ident
    }

    pub fn return_type(&self) -> TokenStream {
        match self.method.sig.decl.output {
            syn::ReturnType::Default => quote!{ () },
            syn::ReturnType::Type(_, ref ty) => quote!{ #ty },
        }
    }

    pub fn call(&self, args: TokenStream) -> TokenStream {

        let ident = &self.method.sig.ident;

        if self.is_static() {
            let ty = &self.item.self_ty;
            quote! { #ty::#ident(#args) }
        } else {
            quote! { self.state.#ident(#args) }
        }
    }

    pub fn fabricate(&self, ret: TokenStream) -> TokenStream {

        let vis = &self.method.vis;
        let defaultness = &self.method.defaultness;
        let sig = &self.method.sig;
        let constness = &sig.constness;
        let unsafety = &sig.unsafety;
        let asyncness = &sig.asyncness;
        let abi = &sig.abi;
        let ident = &sig.ident;
        let decl = &sig.decl;
        let fn_token = &decl.fn_token;
        let generics = &decl.generics;
        let inputs = &decl.inputs;
        let variadic = &decl.variadic;

        quote! { #vis #defaultness #constness #unsafety #asyncness #abi #fn_token #ident #generics ( #inputs #variadic ) -> #ret }
    }
}

pub struct StateMethods<'a> {
    methods: Vec<StateMethod<'a>>
}

impl<'a> StateMethods<'a> {
    pub fn new(item: &'a syn::ItemImpl) -> StateMethods<'a> {

        let methods = item.items.iter()
            .filter_map(|x| match x {
               syn::ImplItem::Method(m) => Some(m),
                _ => None,
            })
            .filter(|m|  match m.vis {
              syn::Visibility::Public(_) => true,
               _ => false,
            })
            .map(|x| StateMethod::new(item, x))
            .collect();

        StateMethods {
            methods: methods,
        }
    }

    pub fn methods(&'a self) -> &'a Vec<StateMethod<'a>> {
        &self.methods
    }
}
