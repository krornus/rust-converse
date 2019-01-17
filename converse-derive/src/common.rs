use proc_macro2::TokenStream;
use quote::quote;
use syn;

type Arguments = syn::punctuated::Punctuated<syn::Pat, syn::token::Comma>;

pub struct PhantomMarker {
    ident: syn::Ident,
    lifetime: Option<syn::LifetimeDef>,
}

impl PhantomMarker {
    fn new(ident: syn::Ident, lifetime: Option<syn::LifetimeDef>) -> Self {
        PhantomMarker {
            ident: ident,
            lifetime: lifetime,
        }
    }

    fn is_reference(&self) -> bool {
        self.lifetime.is_some()
    }

    /* T: 'a */
    fn bounded(&self) -> TokenStream {
        if self.lifetime.is_some() {
            let lt = self.lifetime.as_ref().unwrap();
            let ident = &self.ident;
            quote! { #ident: #lt }
        } else {
            let ident = &self.ident;
            quote! { #ident }
        }
    }

    /* PhantomData */
    fn instance(&self) -> TokenStream {
        quote! { ::std::marker::PhantomData }
    }

    /* T */
    fn ident(&self) -> TokenStream {
        let id = &self.ident;
        quote! { #id }
    }

    /* PhantomData<&'a T> */
    fn decl(&self) -> TokenStream {
        let param = self.decl_param();
        quote! { ::std::marker::PhantomData < #param > }
    }

    /* &'a T */
    fn decl_param(&self) -> TokenStream {
        if self.lifetime.is_some() {
            let lt = self.lifetime.as_ref().unwrap();
            let ident = &self.ident;
            quote! { & #lt #ident }
        } else {
            let ident = &self.ident;
            quote! { #ident }
        }
    }
}

pub struct PhantomMarkers {
    markers: Vec<PhantomMarker>,
}

impl PhantomMarkers {
    fn new(generics: &syn::Generics) -> Self {
        PhantomMarkers {
            markers: Self::mark(generics.clone()),
        }
    }
}

impl PhantomMarkers {
    fn mark(generics: syn::Generics) -> Vec<PhantomMarker> {

        let mut markers = vec![];

        let mut lifetimes = vec![];
        let mut types = vec![];
        let mut type_ids = vec![];
        for gen in generics.params.into_iter() {
            match gen {
                syn::GenericParam::Type(x) => {
                    type_ids.push(x.ident.clone());
                    types.push(x);
                },
                syn::GenericParam::Lifetime(x) => lifetimes.push(x),
                _ => eprintln!("Constant generic types are currently unsupported for Converse"),
            }
        }

        for ty in types.iter() {
            markers.push(PhantomMarker::new(ty.ident.clone(), None));
        }

        /* autogenerate new types which are valid for `lifetime` lifetime */
        let mut ty_idx = 0;
        while !lifetimes.is_empty() {

            let lifetime = lifetimes.pop().unwrap();
            let id = syn::Ident::new(&format!("PhantomType_{}", ty_idx), proc_macro2::Span::call_site());

            /*
             * this will probably never occur unless
             * someone is trying to be a dick
             */
            if type_ids.contains(&&id) {
                ty_idx += 1;
                lifetimes.push(lifetime);
                continue;
            }

            markers.push(PhantomMarker::new(id, Some(lifetime)));
        }

        markers
    }

    /* PhantomType_0: 'a, PhantomType_1: 'b */
    fn generics(&self) -> Vec<TokenStream> {
        self.markers.iter()
            /* only the newly created types */
            .filter(|x| x.is_reference())
            .map(|x| x.bounded())
            .collect()
    }

    /* PhantomType_0, PhantomType_1 */
    fn types(&self) -> Vec<TokenStream> {
        self.markers.iter()
            /* only the newly created types */
            .filter(|x| x.is_reference())
            .map(|x| x.ident())
            .collect()
    }

    /* PhantomData<T>, PhantomData<PhantomType_0>, PhantomData<&'a PhantomType_1> */
    pub fn decls(&self) -> Vec<TokenStream> {
        self.markers.iter()
            .map(|x| x.decl())
            .collect()
    }

    /* PhantomData, PhantomDatam, PhantomData */
    pub fn instances(&self) -> Vec<TokenStream> {
        self.markers.iter()
            .map(|x| x.instance())
            .collect()
    }
}

pub struct TypeSystem<'a> {
    markers: PhantomMarkers,
    generics: &'a syn::Generics,
}

impl<'a> TypeSystem<'a> {
    fn new(generics: &'a syn::Generics) -> Self {
        TypeSystem {
            markers: PhantomMarkers::new(generics),
            generics: generics,
        }
    }

    pub fn markers(&self) -> &PhantomMarkers {
        &self.markers
    }

    /* 'a, T: 'a */
    pub fn generics(&self) -> TokenStream {
        let gen = &self.generics;
        quote!( #gen )
    }

    /* 'a, T: 'a, PhantomType_0: 'a */
    pub fn marked_generics(&self) -> TokenStream {
        let params = &self.generics.params;
        let where_clause = &self.generics.where_clause;
        let marked_params = self.markers.generics().iter()
            .fold(quote!(#params), |acc, tok| quote! { #acc, #tok });
        quote! { < #marked_params > #where_clause }
    }

    /* 'a, T */
    pub fn params(&self) -> Vec<TokenStream> {
        let mut lf = self.lifetimes();
        lf.extend_from_slice(&self.types());
        lf
    }

    /* 'a, T, PhantomType_0 */
    pub fn marked_params(&self) -> Vec<TokenStream> {
        let mut params = self.params();
        let marks = self.markers.types();
        params.extend_from_slice(&marks);

        params
    }

    pub fn lifetimes(&self) -> Vec<TokenStream> {
        self.generics.params.iter()
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
        self.generics.params.iter()
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

}

pub struct StateImpl<'a> {
    item: &'a syn::ItemImpl,
    types: TypeSystem<'a>,
}

impl<'a> StateImpl<'a> {
    pub fn new(item: &'a syn::ItemImpl) -> StateImpl {
        StateImpl {
            item: item,
            types: TypeSystem::new(&item.generics),
        }
    }

    pub fn ty(&self) -> TokenStream {
        let ty = &self.item.self_ty;
        quote! { #ty }
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

    pub fn types(&'a self) -> &'a TypeSystem<'a> {
        &self.types
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
