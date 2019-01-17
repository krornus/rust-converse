use std::env;

use converse;
use converse_derive::Converse;

mod derived;

struct Playlist<'a, T: 'a> {
    list: Vec<String>,
    pub data: &'a T,
}

impl<'a,T> Playlist<'a, T> {
    pub fn new(data: &'a T) -> Playlist<T> {
        Playlist {
            list: vec![],
            data: data,
        }
    }
}

// #[Converse(playlist)]
impl<'a, T> Playlist<'a, T> {
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
        //println!("usage: converse <server|client>");
        return Ok(());
    }

    if argv[1] == "server" {
        let i = 0;
        let playlist = Playlist::new(&i);
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

