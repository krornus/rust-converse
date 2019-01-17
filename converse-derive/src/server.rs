use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn;

use crate::structure::Structure;

pub struct Server {
    structure: Structure,
    directory: String,
}

impl Server {
    pub fn new(item: &syn::ItemImpl, directory: String) -> Self {

        let ident = syn::Ident::new("Server", proc_macro2::Span::call_site());
        let mut structure = Structure::from_impl(ident, item.clone());
        let state_ty = &item.self_ty;

        structure.member(quote! { proc: ::converse::procdir::ProcessDirectory });
        structure.member(quote! { socket: ::std::os::unix::net::UnixListener  });
        structure.member(quote! { state: #state_ty  });

        Server {
            structure,
            directory,
        }
    }
}

impl Server {
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
        fields.push( quote! { socket: socket } );
        fields.push( quote! { state: self } );

        let auto = self.structure.generics().generated();
        /* this actually creates the struct */
        let server = self.structure.initialize(fields);

        let body = quote! {
            fn server<#auto>(self) -> Result<#ty, ::converse::error::Error> {

                let proc = ::converse::procdir::ProcessDirectory::new(#dir)?;
                proc.lock()?;

                let socket = ::std::os::unix::net::UnixListener::bind(proc.socket())?;

                Ok(#server)
            }
        };

        self.structure.implement_parent(body)
    }

    fn implementations(&self) -> TokenStream {

        let core = self.core();
        let endpoints = self.endpoints();

        let body = quote! {
            #core
            #endpoints
        };

        self.structure.implement(body)
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

        let imp = self.structure.implementation();
        let arms = imp.methods().iter().enumerate().map(|(i,x)| {

            let idx = i as u32 + 1;

            let args = (0..x.args().len())
                .map(|i| quote! {
                    ::converse::serde_cbor::from_slice(&req.argv[#i].data)?
                }).collect();

            let ident = x.ident();

            let call = x.call(None, args);
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

        let imp = self.structure.implementation();
        imp.methods().iter().map(|x| {

            let args = x.args().iter().map(|x| quote! { #x }).collect();
            let call = x.call(Some(quote! { state . }), args);

            x.decl(x.ret(), call)

        }).collect()
    }
}

