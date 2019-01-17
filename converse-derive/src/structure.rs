use quote::quote;
use proc_macro2::TokenStream;
use syn::{Attribute, FnDecl, LifetimeDef, Ident, ImplItem, ImplItemMethod, ItemImpl, Visibility, WhereClause};
use syn::token::{Comma, Default};
use syn::punctuated::Punctuated;

pub struct Structure {
    ident: Ident,
    generics: PhantomGenerics,
    members: Punctuated<Ident, TokenStream>,
    imp: Implementation,
    phantoms: Vec<Ident>,
    autovar: usize,
}

impl Structure {
    pub fn from_impl<'a>(ident: Ident, imp: &'a ItemImpl) -> Self {

        let mut structure = Structure {
            ident: ident.clone(),
            generics: PhantomGenerics::from_impl(imp),
            members: Punctuated::new(),
            imp: Implementation::from_impl(ident, imp),
            phantoms: vec![],
            autovar: 0,
        };

        let phantoms = (0..structure.generics.len()).map(|_|structure.autovar()).collect();
        structure.phantoms = phantoms;

        structure
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

    pub fn implement<F>(&self, cb: F) -> TokenStream
    where
        F: Fn(FnDecl) -> TokenStream
    {

        let ident = &self.ident;

        let gen_decls = self.generics.decls();
        let gen_params = self.generics.params();

        let functions: TokenStream = self.imp.functions(cb).into_iter().collect();

        quote! {
            impl #gen_decls #ident #gen_params {
                #functions
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
    methods: Vec<ImplItemMethod>,
}

impl Implementation {
    fn from_impl<'a>(ident: Ident, imp: &'a ItemImpl) -> Self {

        let methods = imp.items.iter()
            .filter_map(|x| match x {
                ImplItem::Method(x) => Some(x.clone()),
                _ => None,
            }).collect();

        Implementation {
            attrs: imp.attrs.clone(),
            ident: ident,
            generics: PhantomGenerics::from_impl(imp),
            methods: methods,
        }
    }

    fn functions<F>(&self, cb: F) -> Vec<TokenStream>
    where
        F: Fn(FnDecl) -> TokenStream
    {
        vec![]
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
                syn::GenericParam::Type(x) => {
                    type_ids.push(&x.ident);
                    generics.push(PhantomGeneric::new(x.ident.clone(), None));
                },
                syn::GenericParam::Lifetime(x) => lifetimes.push(x),
                _ => eprintln!("Constant generic types are currently unsupported for Converse"),
            }
        }

        /* autogenerate new types which are valid for `lifetime` lifetime */
        /* seperate iteration so we have a guarentee for no conflicting types */
        let mut ty_idx = 0;
        while !lifetimes.is_empty() {

            let lifetime = lifetimes.pop().unwrap();
            let id = syn::Ident::new(&format!("PhantomType_{}", ty_idx), proc_macro2::Span::call_site());

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
