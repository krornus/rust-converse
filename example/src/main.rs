use std::env;

use converse;
use converse_derive::Converse;
use converse::serde::{Serialize, de::DeserializeOwned};

struct Playlist<T>
where
    T: Serialize + Clone + DeserializeOwned
{
    list: Vec<String>,
    pub data: T,
}

impl<T: Serialize + Clone + DeserializeOwned> Playlist<T> {
    pub fn new(data: T) -> Playlist<T> {
        Playlist {
            list: vec![],
            data: data,
        }
    }
}

#[Converse(playlist)]
impl<T: Serialize + Clone + DeserializeOwned> Playlist<T> {
    pub fn add(&mut self, x: String) {
        self.list.push(x);
    }

    pub fn get(&mut self, i: usize, default: String) -> String {
        self.list.get(i).map(|x| x.clone()).unwrap_or(default)
    }

    pub fn list(&self) -> Vec<String> {
        self.list.clone()
    }

    pub fn data(&self) -> T {
        self.data.clone()
    }
}

fn main() {
    match run() {
        Ok(_) => {},
        Err(e) => { eprintln!("\x1b[1;31m[-]\x1b[m {}", e); },
    }
}

fn run() -> Result<(), converse::error::Error> {

    let argv: Vec<_> = env::args().collect();

    if argv.len() < 2 {
        println!("usage: converse <server|client>");
        return Ok(());
    }

    if argv[1] == "server" {
        let i: usize = 1234;
        let playlist = Playlist::new(i);
        playlist.server()?.run()?;
        return Ok(())
    }

    {
        let mut playlist = Playlist::<usize>::client()?;

        playlist.add("Client 1".to_string())?;
        println!("list: {:?}", playlist.list()?);
        println!("data: {:?}", playlist.data()?);
    }

    {
        let mut playlist = Playlist::<usize>::client()?;

        playlist.add("Client 2".to_string())?;
        println!("list: {:?}", playlist.list()?);
        println!("data: {:?}", playlist.data()?);
        playlist.exit()?;
    }

    Ok(())
}

