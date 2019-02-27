use proc_macro2::TokenStream;
use quote::quote;
use syn;

use crate::structure::Structure;

pub struct Client {
    structure: Structure,
    directory: String,
}

impl Client {
    pub fn new(item: &syn::ItemImpl, directory: String) -> Self {

        let ident = syn::Ident::new("Client", proc_macro2::Span::call_site());
        let mut structure = Structure::from_impl(ident, item.clone());

        structure.member(quote! { proc: ::converse::procdir::ProcessDirectory });

        Client {
            structure: structure,
            directory: directory,
        }
    }
}

impl Client {
    /* server struct and impls */
    pub fn tokens(&self) -> TokenStream {

        let decl = self.structure.declare();
        let initializer = self.initializer();
        let implementations = self.implementations();

        quote! {
            #decl
            #initializer
            #implementations
        }
    }

    fn initializer(&self) -> TokenStream {

        let dir = &self.directory;
        let ty = self.structure.ty();

        /* proc is declared below in the client function */
        let mut fields = syn::punctuated::Punctuated::new();
        fields.push( quote! { proc: proc } );

        let auto = self.structure.generics().generated();
        /* this actually creates the struct */
        let client = self.structure.initialize(fields);

        let body = quote! {
            pub fn client<#auto>() -> Result<#ty, ::converse::error::Error> {
                let proc = ::converse::procdir::ProcessDirectory::new(#dir)?;

                if !proc.socket().exists() {
                    return Err(::converse::error::Error::Client(
                        format!("Socket file '{}' does not exist.", proc.socket().display())));
                }

                if let Ok(()) = proc.lock() {
                    return Err(::converse::error::Error::Server(
                        format!("Process {} is not locked ('{}').", #dir, proc.lockfile().display())));
                }

                Ok(#client)
            }
        };

        self.structure.implement_parent(body)
    }

    fn implementations(&self) -> TokenStream {

        let endpoints = self.endpoints();

        let body = quote! {
            fn exit(&mut self) -> Result<(), ::converse::error::Error> {
                let mut stream = std::os::unix::net::UnixStream::connect(self.proc.socket())?;
                ::converse::protocol::IPCRequest::new(0, vec![]).write(&mut stream)?;
                Ok(())
            }

            #endpoints
        };

        self.structure.implement(body)
    }

    fn endpoints(&self) -> TokenStream {

        let imp = self.structure.implementation();

        /*
         * for each method, make a new method of the same name
         * which connects, deserializes args, serializes result
         */
        imp.methods().iter().enumerate().map(|(i,x)| {

            let ret = x.ret();
            let args = x.args();

            let idx = (i + 1) as u32;
            let argc = args.len();

            let init = quote! { let mut argv = Vec::with_capacity(#argc); };

            let argv = args.iter()
                .map(|arg| quote! {
                    argv.push(::converse::serde_cbor::to_vec(&#arg)?);
                })
                .fold(init, |acc, tok| quote! {
                     #acc #tok
                 });

            let body = quote! {
                let mut stream = std::os::unix::net::UnixStream::connect(self.proc.socket())?;

                #argv

                ::converse::protocol::IPCRequest::new(#idx, argv).write(&mut stream)?;
                let res = ::converse::protocol::IPCBuffer::read(&mut stream)?;

                Ok(::converse::serde_cbor::from_slice(&res.data)?)
            };


            x.decl(quote! { Result<#ret, ::converse::error::Error> }, body)

        }).collect()
    }
}

