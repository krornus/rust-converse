use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn;

use crate::common;

pub struct Server<'a> {
    state: common::StateImpl<'a>,
    methods: common::StateMethods<'a>,
    directory: String,
}

impl<'a> Server<'a> {
    pub fn new(item: &'a syn::ItemImpl, directory: String) -> Self {

        let state = common::StateImpl::new(item);
        let methods = common::StateMethods::new(item);

        Server {
            state,
            methods,
            directory,
        }
    }
}

impl<'a> Server<'a> {
    pub fn tokens(&self) -> TokenStream {

        let initializer = self.initializer();
        let implementations = self.implementations();

        let gen = self.state.types().generics();
        let ty = quote!{ Server #gen };
        let state = self.state.ty();

        quote! {
            struct #ty {
                state: #state,
                proc: ::converse::procdir::ProcessDirectory,
                socket: ::std::os::unix::net::UnixListener,
            }

            #initializer
            #implementations
        }
    }

    fn initializer(&self) -> TokenStream {

        let dir = &self.directory;
        let impl_state = self.state.implement();
        let params: TokenStream = self.state.types().params().into_iter()
            .map(|x| quote!{ #x , })
            .collect();
        let gen = quote!{ < #params > };
        let ty = quote! { Server #gen };

        quote! {
            #impl_state {
                fn server(self) -> Result<#ty, ::converse::error::Error> {
                    let proc = ::converse::procdir::ProcessDirectory::new(#dir)?;
                    proc.lock()?;

                    let socket = ::std::os::unix::net::UnixListener::bind(proc.socket())?;

                    Ok(Server {
                        state: self,
                        proc: proc,
                        socket: socket,
                    })
                }
            }
        }
    }

    fn implementations(&self) -> TokenStream {

        let core = self.core();
        let endpoints = self.endpoints();
        let params: TokenStream = self.state.types().params().into_iter()
            .map(|x| quote!{ #x , })
            .collect();

        let gen = quote!{ < #params > };
        let ty = quote! { Server #gen };
        let impl_server = self.state.fabricate(ty);

        quote!{
            #impl_server {
                #core
                #endpoints
            }
        }
    }

    fn core(&self) -> TokenStream {

        let matches = self.handle_arms();

        quote! {
            fn run(&mut self) -> Result<(), ::converse::error::Error> {

                let dir = self.proc.path().clone();
                ::converse::ctrlc::set_handler(move || {
                    if dir.exists() {
                        ::std::fs::remove_dir_all(dir.clone())
                            .expect("failed to remove server process directory");
                        ::std::process::exit(0);
                    }
                }).expect("Failed to set interrupt handler for server");


                loop {
                    let (stream, _) = self.socket.accept()?;
                    self.handle(stream)?;
                }
            }

            fn handle(&mut self, mut stream: ::std::os::unix::net::UnixStream) -> Result<(), ::converse::error::Error> {

                let req = ::converse::protocol::IPCRequest::read(&mut stream)?;

                match req.key {
                    0u32 => {
                        self.exit();
                    },
                    #matches
                    _ => {
                        return Err(::converse::error::Error::Server(format!("Invalid function called")));
                    },
                }

                Ok(())
            }

            fn exit(&mut self) {
                self.proc.close();
                ::std::process::exit(0);
            }
        }
    }

    fn handle_arms(&self) -> TokenStream {

        let arms = self.methods.methods().iter().enumerate().map(|(i,x)| {

            let idx = i as u32 + 1;

            let args = (0..x.arguments().len())
                .map(|i| quote! {
                    ::converse::serde_cbor::from_slice(&req.argv[#i].data)?,
                })
                .fold(quote!(), |acc, tok| quote! { #acc #tok });

            let ident = x.ident();

            let call = x.call(args);
            let ret = quote! { let ret = #call; };

            quote_spanned! { ident.span()=>
                #idx => {
                    #ret
                    ::converse::protocol::IPCBuffer::new(::converse::serde_cbor::to_vec(&ret)?).write(&mut stream)?;
                }
            }

        });

        quote!(#(#arms,)*)
    }


    fn endpoints(&self) -> TokenStream {

        self.methods.methods().iter().map(|x| {

            let sig = x.fabricate(x.return_type());

            let args = x.arguments();
            let call = x.call(quote!{ #args });

            quote! {
                #sig { #call }
            }

        }).fold(quote!(), |acc, tok| quote!{ #acc #tok })
    }
}

