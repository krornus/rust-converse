use proc_macro2::TokenStream;
use quote::quote;
use syn;

use crate::common;

pub struct Client<'a> {
    state: common::StateImpl<'a>,
    methods: common::StateMethods<'a>,
    directory: String,
}

impl<'a> Client<'a> {
    pub fn new(item: &'a syn::ItemImpl, directory: String) -> Self {

        let state = common::StateImpl::new(item);
        let methods = common::StateMethods::new(item);

        Client {
            state,
            methods,
            directory,
        }
    }
}

impl<'a> Client<'a> {
    /* server struct and impls */
    pub fn tokens(&self) -> TokenStream {

        let initializer = self.initializer();
        let implementations = self.implementations();

        let gen = self.state.types().marked_generics();
        let ty = quote!{ Client #gen };

        let phantom = self.state.types().markers();
        let markers: TokenStream = phantom.decls().iter().enumerate()
            .map(|(i,x)| {
                let id = syn::Ident::new(&format!("marker_{}", i), proc_macro2::Span::call_site());
                quote! { #id: #x , }
            }).collect();

        quote! {
            struct #ty {
                proc: ::converse::procdir::ProcessDirectory,
                #markers
            }

            #initializer
            #implementations
        }
    }

    fn initializer(&self) -> TokenStream {
        let dir = &self.directory;
        let impl_state = self.state.implement();

        let params: TokenStream = self.state.types().marked_params().into_iter()
            .map(|x| quote!{ #x , })
            .collect();
        let gen = quote! { < #params > };
        let ty = quote! { Client #gen };


        let phantom = self.state.types().markers();
        let markers: TokenStream = phantom.instances().iter().enumerate()
            .map(|(i,x)| {
                let id = syn::Ident::new(&format!("marker_{}", i), proc_macro2::Span::call_site());
                quote! { #id: #x , }
            }).collect();

        quote! {
            #impl_state {
                fn client() -> Result<#ty, ::converse::error::Error> {
                    let proc = ::converse::procdir::ProcessDirectory::new(#dir)?;

                    if !proc.socket().exists() {
                        return Err(::converse::error::Error::Server(
                            format!("Client error: socket file '{}' does not exist.", proc.socket().display())));
                    }

                    if let Ok(()) = proc.lock() {
                        return Err(::converse::error::Error::Server(
                            format!("Client error: process {} is not locked ('{}').", #dir, proc.lockfile().display())));
                    }

                    Ok(Client {
                        proc: proc,
                        #markers
                    })
                }
            }
        }
    }

    fn implementations(&self) -> TokenStream {

        let params: TokenStream = self.state.types().marked_params().into_iter()
            .map(|x| quote!{ #x , })
            .collect();
        let gen = quote!{ < #params > };
        let ty = quote! { Client #gen };
        let impl_client = self.state.fabricate(ty);

        let endpoints = self.endpoints();

        quote! {
            #impl_client {
                fn exit(&mut self) -> Result<(), ::converse::error::Error> {
                    let mut stream = std::os::unix::net::UnixStream::connect(self.proc.socket())?;
                    ::converse::protocol::IPCRequest::new(0, vec![]).write(&mut stream)?;
                    Ok(())
                }
                #endpoints
            }
        }
    }

    fn endpoints(&self) -> TokenStream {

        self.methods.methods().iter().enumerate().map(|(i,x)| {

            let output = x.return_type();
            let ret = quote!{ Result<#output, ::converse::error::Error> };
            let sig = x.fabricate(ret);

            let args = x.arguments();
            let idx = (i + 1) as u32;

            let argc = args.len();
            let argv = args.iter().map(|arg| {
                quote! {
                    argv.push(::converse::serde_cbor::to_vec(&#arg)?);
                }
            }).fold(quote! { let mut argv = Vec::with_capacity(#argc); }, |acc, tok| { quote! { #acc #tok } });

            quote! {
                #sig {
                    let mut stream = std::os::unix::net::UnixStream::connect(self.proc.socket())?;

                    #argv

                    ::converse::protocol::IPCRequest::new(#idx, argv).write(&mut stream)?;
                    let res = ::converse::protocol::IPCBuffer::read(&mut stream)?;

                    Ok(::converse::serde_cbor::from_slice(&res.data)?)
                }
            }

        }).collect()
    }
}

