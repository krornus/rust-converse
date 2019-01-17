
use quote::quote;
use proc_macro2::TokenStream;

use syn::{
    FnArg, GenericParam, LifetimeDef,
    Ident, ImplItem, ImplItemMethod, ItemImpl, Pat,
    ReturnType, Type, WhereClause
};
use syn::token::{Eq, Add, Comma};
use syn::punctuated::{Pair, Punctuated};

pub struct Structure {
    ident: Ident,
    generics: PhantomGenerics,
    members: Punctuated<TokenStream, Comma>,
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
            imp: Implementation::from_impl(&imp),
            parent: imp,
            phantoms: vec![],
            autovar: 0,
        };

        let phantoms = (0..structure.generics.len()).map(|_|structure.autovar()).collect();
        structure.phantoms = phantoms;

        structure
    }

    pub fn member(&mut self, ty: TokenStream) {
        self.members.push(ty);
    }

    pub fn generics(&self) -> &PhantomGenerics {
        &self.generics
    }

    /* impl on the originaly given type */
    pub fn implement_parent(&self, body: TokenStream) -> TokenStream {

        let attrs: TokenStream = self.parent.attrs.iter().map(|x| quote!{ #x }).collect();
        let defaultness = &self.parent.defaultness;
        let unsafety = &self.parent.unsafety;
        let ty = &self.parent.self_ty;
        let params = &self.parent.generics.params;
        let where_clause = &self.parent.generics.where_clause;

        quote! {
            #attrs impl #defaultness #unsafety < #params > #ty #where_clause {
                #body
            }
        }
    }

    pub fn ty(&self) -> TokenStream {

        let ident = &self.ident;
        let gen_params = self.generics.params();

        quote! { #ident < #gen_params > }
    }

    pub fn declare(&self) -> TokenStream {

        let ident = &self.ident;
        let gen_decls = self.generics.decls();

        assert!(self.phantoms.len() == self.generics.len());
        let markers: TokenStream = self.phantoms.iter()
            .zip(self.generics.markers().iter())
            .map(|(id,mark)| {
                quote! { #id: #mark , }
            }).collect();

        let members = &self.members;

        quote! {
            pub struct #ident < #gen_decls > {
                #markers
                #members
            }
        }
    }

    pub fn initialize(&self, mut fields: Punctuated<TokenStream, Comma>) -> TokenStream {

        let ident = &self.ident;

        assert!(self.phantoms.len() == self.generics.len());
        let markers: Vec<TokenStream> = self.phantoms.iter()
            .zip(self.generics.instances().iter())
            .map(|(id,instance)| {
                quote! { #id: #instance }
            }).collect();

        fields.extend(markers.into_iter());

        quote! {
            #ident {
                #fields
            }
        }
    }

    pub fn implement(&self, body: TokenStream) -> TokenStream {

        let ident = &self.ident;

        let gen_decls = self.generics.decls();
        let gen_params = self.generics.params();
        let where_clause = &self.generics.where_clause;

        quote! {
            impl < #gen_decls > #ident < #gen_params >
                #where_clause
            {
                #body
            }
        }
    }

    pub fn implementation(&self) -> &Implementation {
        &self.imp
    }

    fn autovar(&mut self) -> Ident {
        let id = Ident::new(&format!("var{}", self.autovar), proc_macro2::Span::call_site());
        self.autovar += 1;
        id
    }
}

pub struct Implementation {
    methods: Vec<Method>,
}

impl Implementation {
    fn from_impl<'a>(imp: &'a ItemImpl) -> Self {

        let methods = imp.items.iter()
            .filter_map(|x| match x {
                ImplItem::Method(x) => Some(x.clone()),
                _ => None,
            })
            .map(|x| Method::new(imp.self_ty.clone(), x)).collect();

        Implementation {
            methods: methods,
        }
    }

    pub fn methods(&self) -> &Vec<Method> {
        &self.methods
    }
}

#[derive(Clone)]
pub struct Method {
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
    pub fn args(&self) -> Punctuated<Pat, Comma> {
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
                        eprintln!("warning: ingored inferred variable {}", quote!(#x));
                        None
                    },
                    FnArg::Ignored(x) => {
                        eprintln!("warning: ingored variable {}", quote!(#x));
                        None
                    },
                    _ => None,
                }
            })
            .collect()
    }

    pub fn ident(&self) -> &Ident {
        &self.method.sig.ident
    }

    /* Get the return type */
    pub fn ret(&self) -> TokenStream {
        match &self.method.sig.decl.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, x) => quote! { #x },
        }
    }

    /* Create a function declaration stream */
    pub fn decl(&self, ret: TokenStream, body: TokenStream) -> TokenStream {

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

        quote! {
            #vis #defaultness #constness #unsafety #asyncness #abi
            fn #ident #generics ( #inputs ) -> #ret {
                #body
            }
        }
    }

    /* Call the function with args */
    pub fn call(&self, path: Option<TokenStream>, args: Punctuated<TokenStream, Comma>) -> TokenStream {

        let ident = &self.method.sig.ident;

        if self.is_static() {
            let ty = &self.ty;
            quote! { #ty :: #path #ident(#args) }
        } else {
            quote! { self . #path #ident(#args) }
        }
    }
}

pub struct PhantomGenerics {
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

                    let punct = x.bounds.iter().map(|x| quote!(#x)).collect();
                    let bounds = PhantomBounds::new(None, punct, x.eq_token.clone(), x.default.clone());

                    type_ids.push(&x.ident);
                    generics.push(PhantomGeneric::new(x.ident.clone(), bounds));
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

            let bounds = PhantomBounds::new(Some(lifetime.clone()), Punctuated::new(), None, None);
            generics.push(PhantomGeneric::new(id, bounds));
        }

        /* lifetimes first */
        generics.reverse();

        PhantomGenerics {
            generics: generics,
            where_clause: imp.generics.where_clause.clone(),
        }
    }

    pub fn generated(&self) -> Punctuated<TokenStream, Comma> {
        self.generics.iter()
            .filter(|x| x.is_reference())
            .map(PhantomGeneric::ident)
            .collect()
    }

    /* 'a, T: 'a , U: 'b */
    fn decls(&self) -> Punctuated<TokenStream, Comma> {
        /* get lifetimes */
        self.generics.iter()
            .filter_map(PhantomGeneric::lifetime)
            .chain(
                /* get types */
                self.generics.iter().map(PhantomGeneric::decl)
            )
            .collect()
    }

    /* 'a, T */
    fn params(&self) -> Punctuated<TokenStream, Comma> {
        self.generics.iter()
            .filter_map(PhantomGeneric::lifetime)
            .chain(
                /* get types */
                self.generics.iter().map(PhantomGeneric::ident)
            )
            .collect()
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
    bounds: PhantomBounds,
}

impl PhantomGeneric {
    fn new(ident: Ident, bounds: PhantomBounds) -> Self {
        PhantomGeneric {
            ident: ident,
            bounds: bounds,
        }
    }

    /* lifetime is only some when constructed as a reference above */
    fn is_reference(&self) -> bool {
        self.bounds.lifetime.is_some()
    }

    /* T: 'a */
    fn decl(&self) -> TokenStream {
        let ident = &self.ident;
        let bounds = self.bounds.bounds();
        quote! { #ident #bounds }
    }

    /* 'a */
    fn lifetime(&self) -> Option<TokenStream> {
        self.bounds.lifetime.as_ref().map(|x| quote! { #x })
    }

    /* T */
    fn ident(&self) -> TokenStream {
        let ident = &self.ident;
        quote! { #ident }
    }

    /* PhantomData<&'a T> */
    fn marker(&self) -> TokenStream {
        let ty = self.marker_type();
        quote! { ::std::marker::PhantomData < #ty > }
    }

    /* &'a T */
    fn marker_type(&self) -> TokenStream {
        if self.bounds.lifetime.is_some() {
            let lt = self.bounds.lifetime.as_ref().unwrap();
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

struct PhantomBounds {
    lifetime: Option<LifetimeDef>,
    bounds: Punctuated<TokenStream, Add>,
    eq_token: Option<Eq>,
    default: Option<Type>,
}

impl PhantomBounds {
    fn new(
        lifetime: Option<LifetimeDef>,
        bounds: Punctuated<TokenStream, Add>,
        eq_token: Option<Eq>,
        default: Option<Type>,
    ) -> Self {
        PhantomBounds {
            lifetime,
            bounds,
            eq_token,
            default,
        }
    }

    fn bounds(&self) -> Option<TokenStream> {
        if self.lifetime.is_none() && self.bounds.is_empty() {
            None
        } else {
            /* prepend the bounds with the explicit lifetime */
            let mut punct: Punctuated<TokenStream, Add> = self.lifetime.iter().map(|x| quote! { #x }).collect();
            punct.extend(self.bounds.clone().into_iter());

            let eq = &self.eq_token;
            let default = &self.default;

            Some(quote! {
                : #punct #eq #default
            })
        }
    }
}
