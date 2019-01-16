use std::env;

use converse;
use converse_derive::Converse;

struct Playlist<T> {
    list: Vec<String>,
    pub data: T,
}

impl<T> Playlist<T> {
    pub fn new(data: T) -> Playlist<T> {
        Playlist {
            list: vec![],
            data: data,
        }
    }
}

//#[Converse(playlist)]
impl<T> Playlist<T> {
    pub fn add(&mut self, x: String) {
        self.list.push(x);
    }

    pub fn get(&mut self, i: usize, default: String) -> String {
        self.list.get(i).map(|x| x.clone()).unwrap_or(default)
    }

    pub fn list(&self) -> Vec<String> {
        self.list.clone()
    }
}

fn main() {
    match run() {
        Ok(_) => {},
        Err(e) => { eprintln!("Error: {}", e); },
    }
}

fn run() -> Result<(), converse::error::Error> {

    let argv: Vec<_> = env::args().collect();

    if argv.len() < 2 {
        println!("usage: converse <server|client>");
        return Ok(());
    }

    if argv[1] == "server" {
        let playlist = Playlist::new(0);
        println!("{}", playlist.data);
        // playlist.server()?.run()?;
        return Ok(())
    }

    // let mut playlist = Playlist::<usize>::client()?;

    // playlist.add("Test".to_string())?;
    // println!("{:?}", playlist.list()?);
    // playlist.exit()?;

    Ok(())
}

struct Server < T > {
state : Playlist < T > , proc : :: converse :: procdir :: ProcessDirectory ,
socket : :: std :: os :: unix :: net :: UnixListener , } impl < T > Playlist <
T > {
fn server ( self ) -> Result < Server < T > , :: converse :: error :: Error >
{
let proc = :: converse :: procdir :: ProcessDirectory :: new ( "playlist" ) ?
; proc . lock (  ) ? ; let socket = :: std :: os :: unix :: net ::
UnixListener :: bind ( proc . socket (  ) ) ? ; Ok (
Server { state : self , proc : proc , socket : socket , } ) } } impl < T >
Server < T > {
fn run ( & mut self ) -> Result < (  ) , :: converse :: error :: Error > {
let dir = self . proc . path (  ) . clone (  ) ; :: converse :: ctrlc ::
set_handler (
move || {
if dir . exists (  ) {
:: std :: fs :: remove_dir_all ( dir . clone (  ) ) . expect (
"failed to remove server process directory" ) ; :: std :: process :: exit ( 0
) ; } } ) . expect ( "Failed to set interrupt handler for server" ) ; loop {
let ( stream , _ ) = self . socket . accept (  ) ? ; self . handle ( stream )
? ; } } fn handle (
& mut self , mut stream : :: std :: os :: unix :: net :: UnixStream ) ->
Result < (  ) , :: converse :: error :: Error > {
let req = :: converse :: protocol :: IPCRequest :: read ( & mut stream ) ? ;
let buf = match req . key {
0u32 => { self . exit (  ) ; } , 1u32 => {
let ret = self . state . add (
:: converse :: serde_cbor :: from_slice ( & req . argv [ 0usize ] . data ) ? ,
) ; :: converse :: protocol :: IPCBuffer :: new (
:: converse :: serde_cbor :: to_vec ( & ret ) ? ) . write ( & mut stream ) ? ;
} , 2u32 => {
let ret = self . state . get (
:: converse :: serde_cbor :: from_slice ( & req . argv [ 0usize ] . data ) ? ,
:: converse :: serde_cbor :: from_slice ( & req . argv [ 1usize ] . data ) ? ,
) ; :: converse :: protocol :: IPCBuffer :: new (
:: converse :: serde_cbor :: to_vec ( & ret ) ? ) . write ( & mut stream ) ? ;
} , 3u32 => {
let ret = self . state . list (  ) ; :: converse :: protocol :: IPCBuffer ::
new ( :: converse :: serde_cbor :: to_vec ( & ret ) ? ) . write ( & mut stream
) ? ; } , _ => {
return Err (
:: converse :: error :: Error :: Server (
format ! ( "Invalid function called" ) ) ) ; } , } ; Ok ( (  ) ) } fn exit (
& mut self ) { self . proc . close (  ) ; :: std :: process :: exit ( 0 ) ; }
pub fn add ( & mut self , x : String ) -> (  ) { self . state . add ( x ) }
pub fn get ( & mut self , i : usize , default : String ) -> String {
self . state . get ( i , default ) } pub fn list ( & self ) -> Vec < String >
{ self . state . list (  ) } } struct Client < T > {
proc : :: converse :: procdir :: ProcessDirectory , } impl < T > Playlist < T
> {
fn client (  ) -> Result < Client < T > , :: converse :: error :: Error > {
let proc = :: converse :: procdir :: ProcessDirectory :: new ( "playlist" ) ?
; if ! proc . socket (  ) . exists (  ) {
return Err (
:: converse :: error :: Error :: Server (
format ! (
"Client error: socket file '{}' does not exist." , proc . socket (  ) .
display (  ) ) ) ) ; } if let Ok ( (  ) ) = proc . lock (  ) {
return Err (
:: converse :: error :: Error :: Server (
format ! (
"Client error: process {} is not locked ('{}')." , "playlist" , proc .
lockfile (  ) . display (  ) ) ) ) ; } Ok ( Client { proc : proc , } ) } }
impl < T > Client < T > {
fn exit ( & mut self ) -> Result < (  ) , :: converse :: error :: Error > {
let mut stream = std :: os :: unix :: net :: UnixStream :: connect (
self . proc . socket (  ) ) ? ; :: converse :: protocol :: IPCRequest :: new (
0 , vec ! [  ] ) . write ( & mut stream ) ? ; Ok ( (  ) ) } pub fn add (
& mut self , x : String ) -> Result < (  ) , :: converse :: error :: Error > {
let mut stream = std :: os :: unix :: net :: UnixStream :: connect (
self . proc . socket (  ) ) ? ; let mut argv = Vec :: with_capacity ( 1usize )
; argv . push ( :: converse :: serde_cbor :: to_vec ( & x ) ? ) ; :: converse
:: protocol :: IPCRequest :: new ( 1u32 , argv ) . write ( & mut stream ) ? ;
let res = :: converse :: protocol :: IPCBuffer :: read ( & mut stream ) ? ; Ok
( :: converse :: serde_cbor :: from_slice ( & res . data ) ? ) } pub fn get (
& mut self , i : usize , default : String ) -> Result < String , :: converse
:: error :: Error > {
let mut stream = std :: os :: unix :: net :: UnixStream :: connect (
self . proc . socket (  ) ) ? ; let mut argv = Vec :: with_capacity ( 2usize )
; argv . push ( :: converse :: serde_cbor :: to_vec ( & i ) ? ) ; argv . push
( :: converse :: serde_cbor :: to_vec ( & default ) ? ) ; :: converse ::
protocol :: IPCRequest :: new ( 2u32 , argv ) . write ( & mut stream ) ? ; let
res = :: converse :: protocol :: IPCBuffer :: read ( & mut stream ) ? ; Ok (
:: converse :: serde_cbor :: from_slice ( & res . data ) ? ) } pub fn list (
& self ) -> Result < Vec < String > , :: converse :: error :: Error > {
let mut stream = std :: os :: unix :: net :: UnixStream :: connect (
self . proc . socket (  ) ) ? ; let mut argv = Vec :: with_capacity ( 0usize )
; :: converse :: protocol :: IPCRequest :: new ( 3u32 , argv ) . write (
& mut stream ) ? ; let res = :: converse :: protocol :: IPCBuffer :: read (
& mut stream ) ? ; Ok (
:: converse :: serde_cbor :: from_slice ( & res . data ) ? ) } }

