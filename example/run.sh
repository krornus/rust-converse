#!/bin/sh

pre="
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
}"

cmod="// mod derived;"
umod="mod derived;"

cder='// #\[Converse\(playlist\)\]'
uder='#\[Converse\(playlist\)\]'

sed -Ei "s:^${umod}:${cmod}:" src/main.rs
sed -Ei "s:^${cder}:${uder}:" src/main.rs

echo $pre > src/derived.rs
cargo run >> src/derived.rs 2> /dev/null
rustfmt src/derived.rs 2> /dev/null

sed -Ei "s:^${cmod}:${umod}:" src/main.rs
sed -Ei "s:^${uder}:${cder}:" src/main.rs
