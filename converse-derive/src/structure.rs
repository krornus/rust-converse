
use quote::quote;
use proc_macro2::TokenStream;

use syn::{
    Attribute, FnArg, FnDecl, GenericParam, LifetimeDef,
    Ident, ImplItem, ImplItemMethod, ItemImpl, Pat,
    ReturnType, Type, Visibility, WhereClause
};
use syn::token::{Comma, Default};
use syn::punctuated::{Pair, Punctuated};

pub struct Structure {
    ident: Ident,
    generics: PhantomGenerics,
    members: Punctuated<Ident, TokenStream>,
    imp: Implementation,
    parent: ItemImpl,
    phantoms: Vec<Ident>,
    autovar: usize,
}

impl Structure {
    pub fn from_impl<'a>(ident: Ident, imp: ItemImpl) -> Self {

        let mut structure = Structure {
            ident: ident.clone(),
            generics: PhantomGenerics::from_impl(&imp),
            members: Punctuated::new(),
            imp: Implementation::from_impl(ident, &imp),
            parent: imp,
            phantoms: vec![],
            autovar: 0,
        };

        let phantoms = (0..structure.generics.len()).map(|_|structure.autovar()).collect();
        structure.phantoms = phantoms;

        structure
    }

    pub fn implement_parent(&self, body: TokenStream) -> TokenStream {
        let attrs: TokenStream = self.parent.attrs.iter().map(|x| quote!{ #x }).collect();
        let defaultness = &self.parent.defaultness;
        let unsafety = &self.parent.unsafety;
        let generics = &self.parent.generics;
        let ty = &self.parent.self_ty;

        quote! {
            #attrs impl #defaultness #unsafety #generics #ty {
                #body
            }
        }
    }

    pub fn declare(&self) -> TokenStream {
        let ident = &self.ident;
        let gen_decls = self.generics.decls();

        assert!(self.phantoms.len() == self.generics.len());

        let markers: TokenStream = self.phantoms.iter()
            .zip(self.generics.markers().iter())
            .map(|(id,mark)| {
                quote! { #id: #mark }
            }).collect();

        let members = &self.members;

        quote! {
            pub struct #ident < #gen_decls > {
                #markers
                #members
            }
        }
    }

    pub fn implement(&self, body: TokenStream) -> TokenStream {

        let ident = &self.ident;

        let gen_decls = self.generics.decls();
        let gen_params = self.generics.params();

        quote! {
            impl < #gen_decls > #ident < #gen_params > {
                #body
            }
        }
    }

    fn autovar(&mut self) -> Ident {
        let id = Ident::new(&format!("var{}", self.autovar), proc_macro2::Span::call_site());
        self.autovar += 1;
        id
    }
}

struct Implementation {
    attrs: Vec<Attribute>,
    ident: Ident,
    generics: PhantomGenerics,
    methods: Vec<Method>,
}

impl Implementation {
    fn from_impl<'a>(ident: Ident, imp: &'a ItemImpl) -> Self {

        let methods = imp.items.iter()
            .filter_map(|x| match x {
                ImplItem::Method(x) => Some(x.clone()),
                _ => None,
            })
            .map(|x| Method::new(imp.self_ty.clone(), x)).collect();

        Implementation {
            attrs: imp.attrs.clone(),
            ident: ident,
            generics: PhantomGenerics::from_impl(imp),
            methods: methods,
        }
    }

    fn methods(&self) -> &Vec<Method> {
        &self.methods
    }
}

struct Method {
    ty: Box<Type>,
    method: ImplItemMethod,
}

impl Method {
    fn new(ty: Box<Type>, method: ImplItemMethod) -> Self {
        Method {
            ty: ty,
            method: method,
        }
    }

    /* Check if this is an instance method */
    fn is_static(&self) -> bool {
        if let Some(pair) = self.method.sig.decl.inputs.first() {
            match pair.value() {
                FnArg::SelfRef(_) => false,
                FnArg::SelfValue(_) => false,
                _ => true,
            }
        } else {
            true
        }
    }

    /* Get a list of arguments to the function - ignore self */
    fn args(&self) -> Punctuated<Pat, Comma> {
        self.method.sig.decl.inputs.pairs()
            .filter(|x| match x.value() {
                FnArg::SelfRef(_) => false,
                FnArg::SelfValue(_) => false,
                _ => true,
            })
            .filter_map(|x| {
                match x.value() {
                    FnArg::Captured(arg) => Some(Pair::new(
                        arg.pat.clone(),
                        x.punct().map(|x| (**x).clone())
                    )),
                    FnArg::Inferred(x) => {
                        eprintln!("warning: ingored inferred variable {}", quote!(x));
                        None
                    },
                    FnArg::Ignored(x) => {
                        eprintln!("warning: ingored variable {}", quote!(x));
                        None
                    },
                    _ => None,
                }
            })
            .collect()
    }

    /* Get the return type */
    fn ret(&self) -> TokenStream {
        match &self.method.sig.decl.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, x) => quote! { #x },
        }
    }

    /* Create the function declaration stream */
    fn decl(&self, ret: TokenStream) -> TokenStream {

        let sig = &self.method.sig;
        let decl = &sig.decl;

        if decl.variadic.is_some() {
            eprintln!("warning: variadic methods are not supported yet");
            return quote!();
        }

        let vis = &self.method.vis;
        let defaultness = &self.method.defaultness;
        let constness = &sig.constness;
        let unsafety = &sig.unsafety;
        let asyncness = &sig.asyncness;
        let abi = &sig.abi;
        let ident = &sig.ident;
        let generics = &decl.generics;
        let inputs = &decl.inputs;

        quote! { #vis #defaultness #constness #unsafety #asyncness #abi fn #ident #generics ( #inputs ) -> #ret }
    }

    /* Call the function with args */
    fn call(&self, args: Punctuated<TokenStream, Comma>) -> TokenStream {

        let ident = &self.method.sig.ident;

        if self.is_static() {
            let ty = &self.ty;
            quote! { #ty::#ident(#args) }
        } else {
            quote! { self.state.#ident(#args) }
        }
    }
}

struct PhantomGenerics {
    generics: Vec<PhantomGeneric>,
    where_clause: Option<WhereClause>,
}

impl PhantomGenerics {
    fn from_impl<'a>(imp: &'a ItemImpl) -> Self {

        let mut generics = vec![];
        let mut lifetimes = vec![];
        let mut type_ids = vec![];

        /* first split types and lifetimes into seperate vectors */
        /* track ids so we can check for conflicts */
        for gen in imp.generics.params.iter() {
            match gen {
                GenericParam::Type(x) => {
                    type_ids.push(&x.ident);
                    generics.push(PhantomGeneric::new(x.ident.clone(), None));
                },
                GenericParam::Lifetime(x) => lifetimes.push(x),
                _ => eprintln!("Constant generic types are currently unsupported for Converse"),
            }
        }

        /* autogenerate new types which are valid for `lifetime` lifetime */
        /* seperate iteration so we have a guarentee for no conflicting types */
        let mut ty_idx = 0;
        while !lifetimes.is_empty() {

            let lifetime = lifetimes.pop().unwrap();
            let id = Ident::new(&format!("PhantomType_{}", ty_idx), proc_macro2::Span::call_site());

            /* conflict */
            if type_ids.contains(&&id) {
                ty_idx += 1;
                lifetimes.push(lifetime);
                continue;
            }

            generics.push(PhantomGeneric::new(id, Some(lifetime.clone())));
        }

        PhantomGenerics {
            generics: generics,
            where_clause: imp.generics.where_clause.clone(),
        }
    }

    /* T: 'a , U: 'b */
    fn decls(&self) -> Punctuated<TokenStream, Comma> {
        self.generics.iter().map(PhantomGeneric::decl).collect()
    }

    /* 'a, T */
    fn params(&self) -> Punctuated<TokenStream, Comma> {
        self.generics.iter().map(PhantomGeneric::ident).collect()
    }

    /* PhantomData<&'a T>, ... */
    fn markers(&self) -> Vec<TokenStream> {
        self.generics.iter().map(PhantomGeneric::marker).collect()
    }

    fn instances(&self) -> Vec<TokenStream> {
        self.generics.iter().map(PhantomGeneric::instance).collect()
    }

    fn len(&self) -> usize {
        self.generics.len()
    }
}

struct PhantomGeneric {
    ident: Ident,
    lifetime: Option<LifetimeDef>,
}

impl PhantomGeneric {
    fn new(ident: Ident, lifetime: Option<LifetimeDef>) -> Self {
        PhantomGeneric {
            ident: ident,
            lifetime: lifetime,
        }
    }

    fn is_reference(&self) -> bool {
        self.lifetime.is_some()
    }

    /* T: 'a */
    fn decl(&self) -> TokenStream {
        if self.lifetime.is_some() {
            let lt = self.lifetime.as_ref().unwrap();
            let ident = &self.ident;
            quote! { #ident: #lt }
        } else {
            let ident = &self.ident;
            quote! { #ident }
        }
    }

    /* If ref: lifetime id, if type: type id */
    fn ident(&self) -> TokenStream {
        if self.lifetime.is_some() {
            let lt = self.lifetime.as_ref().unwrap();
            quote! { #lt }
        } else {
            let ident = &self.ident;
            quote! { #ident }
        }
    }

    /* PhantomData<&'a T> */
    fn marker(&self) -> TokenStream {
        let ty = self.marker_type();
        quote! { ::std::marker::PhantomData < #ty > }
    }

    /* &'a T */
    fn marker_type(&self) -> TokenStream {
        if self.lifetime.is_some() {
            let lt = self.lifetime.as_ref().unwrap();
            let ident = &self.ident;
            quote! { & #lt #ident }
        } else {
            let ident = &self.ident;
            quote! { #ident }
        }
    }

    /* PhantomData */
    fn instance(&self) -> TokenStream {
        quote! { ::std::marker::PhantomData }
    }
}
