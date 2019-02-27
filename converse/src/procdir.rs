use std::fs::{self,File};
use std::io::prelude::*;

use std::env;
use std::process;
use std::path::PathBuf;

use crate::error::Error;

pub struct ProcessDirectory {
    path:   PathBuf,
    lock:   PathBuf,
    socket: PathBuf,
}

impl ProcessDirectory {
    pub fn new<'a>(name: &'a str) -> Result<Self, Error> {
        let path = Self::mkpath(name)?;

        let proc = ProcessDirectory {
            lock: path.join("lock"),
            socket: path.join("socket"),
            path: path,
        };

        if !proc.path.exists() {
            fs::create_dir(&proc.path)?;
        }

        Ok(proc)
    }

    fn mkpath<'a>(name: &'a str) -> Result<PathBuf, Error> {

        let tmp = env::temp_dir();

        if !tmp.exists() {
            return Err(Error::ProcessDirectory(
                format!("Invalid temporary directory found: '{}'", tmp.display())))
        }

        if !tmp.is_dir() {
            return Err(Error::ProcessDirectory(
                format!("Temporary directory is not a valid directory: '{}'", tmp.display())))
        }

        let path = tmp.join(name);

        if !path.is_absolute() {
            return Err(Error::ProcessDirectory(
                format!("Server path must be absolute: '{}'", path.display())))
        }

        Ok(path)
    }

    /* load PID from file */
    pub fn read_pid(&self) -> Result<u32, Error> {

        let mut pid = String::new();
        let mut file = File::open(&self.lock)?;

        file.read_to_string(&mut pid)?;

        match pid.parse() {
            Ok(x) => Ok(x),
            Err(_) => Err(Error::ProcessDirectory(
                format!("Invalid PID given in lock file: '{}'", pid)))
        }
    }

    pub fn lock(&self) -> Result<(), Error> {

        /* can't lock if the a different pid is in the lockfile */
        let pid = self.read_pid();
        if pid.is_ok() && pid.unwrap() != process::id() {
            return Err(Error::ProcessDirectory(
                format!("Failed to lock process directory: '{}'", self.path.display())
            ))
        }

        /* either getting pid from file failed or it is corrupted */
        if self.lock.exists() {
            fs::remove_file(&self.lock)?;
        }

        let pid = process::id().to_string();
        let mut file = File::create(&self.lock)?;
        file.write_all(pid.as_bytes())?;
        file.sync_data()?;

        Ok(())
    }

    pub fn path<'a>(&'a self) -> &'a PathBuf {
        &self.path
    }

    pub fn lockfile<'a>(&'a self) -> &'a PathBuf {
        &self.lock
    }

    pub fn socket<'a>(&'a self) -> &'a PathBuf {
        &self.socket
    }

    pub fn close(&mut self) {
        let pid = self.read_pid().unwrap_or(0) == process::id();

        if pid {
            fs::remove_file(&self.lock).map_err(|e| {
                // warn!("Failed to remove process lock file: '{}'\n\t{}", self.lock.display(), e);
            }).ok();

            fs::remove_file(&self.socket).map_err(|e| {
                // warn!("Failed to remove process lock file: '{}'\n\t{}", self.socket.display(), e);
            }).ok();

            fs::remove_dir(&self.path).map_err(|e| {
                // warn!("Failed to remove temp process directory: '{}'\n\t{}", self.path.display(), e);
            }).ok();
        }

    }
}
