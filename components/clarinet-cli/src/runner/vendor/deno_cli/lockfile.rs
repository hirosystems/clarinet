// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use deno_core::parking_lot::Mutex;
use deno_core::serde_json;
use deno_core::serde_json::json;
use deno_core::ModuleSpecifier;
use log::debug;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Result;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use super::tools::fmt::format_json;

#[derive(Debug, Clone)]
pub struct Lockfile {
    write: bool,
    map: BTreeMap<String, String>,
    pub filename: PathBuf,
}

impl Lockfile {
    pub fn new(filename: PathBuf, write: bool) -> Result<Lockfile> {
        let map = if write {
            BTreeMap::new()
        } else {
            let s = std::fs::read_to_string(&filename)?;
            serde_json::from_str(&s)?
        };

        Ok(Lockfile {
            write,
            map,
            filename,
        })
    }

    // Synchronize lock file to disk - noop if --lock-write file is not specified.
    pub fn write(&self) -> Result<()> {
        if !self.write {
            return Ok(());
        }
        let j = json!(&self.map);
        let s = serde_json::to_string_pretty(&j).unwrap();

        let format_s = format_json(&s, &Default::default())
            .ok()
            .flatten()
            .unwrap_or(s);
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.filename)?;
        use std::io::Write;
        f.write_all(format_s.as_bytes())?;
        debug!("lockfile write {}", self.filename.display());
        Ok(())
    }

    pub fn check_or_insert(&mut self, specifier: &str, code: &str) -> bool {
        if self.write {
            // In case --lock-write is specified check always passes
            self.insert(specifier, code);
            true
        } else {
            self.check(specifier, code)
        }
    }

    /// Checks the given module is included.
    /// Returns Ok(true) if check passed.
    fn check(&mut self, specifier: &str, code: &str) -> bool {
        if specifier.starts_with("file:") {
            return true;
        }
        if let Some(lockfile_checksum) = self.map.get(specifier) {
            let compiled_checksum = super::checksum::gen(&[code.as_bytes()]);
            lockfile_checksum == &compiled_checksum
        } else {
            false
        }
    }

    fn insert(&mut self, specifier: &str, code: &str) {
        if specifier.starts_with("file:") {
            return;
        }
        let checksum = super::checksum::gen(&[code.as_bytes()]);
        self.map.insert(specifier.to_string(), checksum);
    }
}

#[derive(Debug)]
pub struct Locker(Option<Arc<Mutex<Lockfile>>>);

impl deno_graph::source::Locker for Locker {
    fn check_or_insert(&mut self, specifier: &ModuleSpecifier, source: &str) -> bool {
        if let Some(lock_file) = &self.0 {
            let mut lock_file = lock_file.lock();
            lock_file.check_or_insert(specifier.as_str(), source)
        } else {
            true
        }
    }

    fn get_checksum(&self, content: &str) -> String {
        super::checksum::gen(&[content.as_bytes()])
    }

    fn get_filename(&self) -> Option<String> {
        let lock_file = self.0.as_ref()?.lock();
        lock_file.filename.to_str().map(|s| s.to_string())
    }
}

pub fn as_maybe_locker(
    lockfile: Option<Arc<Mutex<Lockfile>>>,
) -> Option<Rc<RefCell<Box<dyn deno_graph::source::Locker>>>> {
    lockfile.as_ref().map(|lf| {
        Rc::new(RefCell::new(
            Box::new(Locker(Some(lf.clone()))) as Box<dyn deno_graph::source::Locker>
        ))
    })
}
