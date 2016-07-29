//! Utility crate to handle the `TARGET` environment variable passed into build.rs scripts.
//!
//! Unlike rust’s `#[cfg(target…)]` attributes, `build.rs`-scripts do not expose a convenient way
//! to detect the system the code will be built for in a way which would properly support
//! cross-compilation.
//!
//! This crate exposes `target_arch`, `target_vendor`, `target_os` and `target_abi` very much in
//! the same manner as the corresponding `cfg` attributes in Rust do, thus allowing `build.rs`
//! script to adjust the output depending on the target the crate is being built for..
//!
//! Custom target json files are also supported.
//!
//! # Usage
//!
//! This crate is only useful if you’re using a build script (`build.rs`). Add dependency to this
//! crate to your `Cargo.toml` via:
//!
//! ```toml
//! [package]
//! # ...
//! build = "build.rs"
//!
//! [build-dependencies]
//! target_build_utils = "0.1"
//! ```
//!
//! Then write your `build.rs` like this:
//!
//! ```rust,no_run
//! extern crate target_build_utils;
//! use target_build_utils::TargetInfo;
//!
//! fn main() {
//!     let target = TargetInfo::new().expect("could not get target info");
//!     if target.target_os() == "windows" {
//!         // conditional stuff for windows
//!     }
//! }
//! ```
//!
//! Now, when running `cargo build`, your `build.rs` should be aware of the properties of the
//! target system when your crate is being cross-compiled.
extern crate serde_json;

use std::env;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::ffi::OsString;

pub struct TargetInfo {
    arch: String,
    vendor: String,
    os: String,
    env: String,
    endian: String,
    pointer_width: String,
}


#[derive(Debug)]
pub enum Error {
    /// The `TARGET` environment variable does not exist or is not valid utf-8
    TargetUnset,
    /// Target was not found
    TargetNotFound,
    /// Custom target JSON was found, but was invalid
    InvalidSpec,
    /// IO error occured during search of JSON target files
    Io(::std::io::Error)
}

impl TargetInfo {
    /// Parse the target info from `TARGET` environment variable
    ///
    /// `TARGET` environment variable is usually set for you in build.rs scripts, therefore this
    /// function is all that’s necessary in majority of cases.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use target_build_utils::TargetInfo;
    /// let target = TargetInfo::new().expect("could not get target");
    /// ```
    pub fn new() -> Result<TargetInfo, Error> {
        env::var("TARGET").map_err(|_| Error::TargetUnset).and_then(|s| TargetInfo::from_str(&s))
    }

    /// Calculate the target info from the provided target value
    ///
    /// String may contain a triple or path to the json file.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use target_build_utils::TargetInfo;
    /// let target = TargetInfo::from_str("x86_64-unknown-linux-gnu")
    ///     .expect("could not get target");
    /// ```
    pub fn from_str(s: &str) -> Result<TargetInfo, Error> {
        fn load_json(path: &Path) -> Result<TargetInfo, Error> {
            use serde_json as s;
            let f = try!(File::open(path).map_err(|e| Error::Io(e)));
            let json: s::Value = try!(s::from_reader(f).map_err(|e| match e {
                s::Error::Io(e) => Error::Io(e),
                _ => Error::InvalidSpec,
            }));
            let req = |name: &str|
                json.find(name).and_then(|a| a.as_str()).ok_or(Error::InvalidSpec);

            Ok(TargetInfo {
                arch: try!(req("arch")).into(),
                os: try!(req("os")).into(),
                vendor: json.find("vendor").and_then(|s| s.as_str()).unwrap_or("unknown").into(),
                env: json.find("env").and_then(|s| s.as_str()).unwrap_or("").into(),
                endian: try!(req("target-endian")).into(),
                pointer_width: try!(req("target-pointer-width")).into(),
            })
        }

        if let Some(t) = TargetInfo::load_specific(s) {
            return Ok(t);
        }
        let path = Path::new(s);
        if path.is_file() {
            return load_json(&path);
        }
        let path = {
            let mut target = String::from(s);
            target.push_str(".json");
            PathBuf::from(target)
        };
        let target_path = env::var_os("RUST_TARGET_PATH")
                              .unwrap_or(OsString::new());
        for dir in env::split_paths(&target_path) {
            let p =  dir.join(&path);
            if p.is_file() {
                return load_json(&p);
            }
        }
        Err(Error::TargetNotFound)
    }

    fn load_specific(s: &str) -> Option<TargetInfo> {
        fn ti(a: &str, v: &str, s: &str, b: &str, e: &str, w: &str) -> Option<TargetInfo> {
            Some(TargetInfo {
                arch: a.into(),
                vendor: v.into(),
                os: s.into(),
                env: b.into(),
                endian: e.into(),
                pointer_width: w.into()
            })
        }
        // Targets known to rustc
        match s {
            "x86_64-unknown-linux-gnu" => ti("x86_64", "unknown", "linux", "gnu", "little", "64"),
            "i686-unknown-linux-gnu" |
            "i586-unknown-linux-gnu" => ti("x86", "unknown", "linux", "gnu", "little", "32"),
            "mips-unknown-linux-gnu" => ti("mips", "unknown", "linux", "gnu", "big", "32"),
            "mipsel-unknown-linux-gnu" => ti("mips", "unknown", "linux", "gnu", "little", "32"),
            "powerpc-unknown-linux-gnu" => ti("powerpc", "unknown", "linux", "gnu", "big", "32"),
            "powerpc64-unknown-linux-gnu"=> ti("powerpc64", "unknown", "linux", "gnu", "big", "64"),
            "powerpc64le-unknown-linux-gnu"=>
                ti("powerpc64", "unknown", "linux", "gnu", "little", "64"),
            "arm-unknown-linux-gnueabi" |
            "arm-unknown-linux-gnueabihf" |
            "armv7-unknown-linux-gnueabihf" =>
                ti("arm", "unknown", "linux", "gnu", "little", "32"),
            "aarch64-unknown-linux-gnu"=> ti("aarch64", "unknown", "linux", "gnu", "little", "64"),
            "x86_64-unknown-linux-musl"=> ti("x86_64", "unknown", "linux", "musl", "little", "64"),
            "i686-unknown-linux-musl"=> ti("x86", "unknown", "linux", "musl", "little", "32"),
            "mips-unknown-linux-musl"=> ti("mips", "unknown", "linux", "musl", "big", "32"),
            "mipsel-unknown-linux-musl"=> ti("mips", "unknown", "linux", "musl", "little", "32"),
            "i686-linux-android"=> ti("x86", "unknown", "android", "", "little", "32"),
            "arm-linux-androideabi" |
            "armv7-linux-androideabi" => ti("arm", "unknown", "android", "", "little", "32"),
            "aarch64-linux-android"=> ti("aarch64", "unknown", "android", "", "little", "64"),
            "i686-unknown-freebsd"=> ti("x86", "unknown", "freebsd", "", "little", "32"),
            "x86_64-unknown-freebsd"=> ti("x86_64", "unknown", "freebsd", "", "little", "64"),
            "i686-unknown-dragonfly"=> ti("x86", "unknown", "dragonfly", "", "little", "32"),
            "x86_64-unknown-dragonfly"=> ti("x86_64", "unknown", "dragonfly", "", "little", "64"),
            "x86_64-unknown-bitrig"=> ti("x86_64", "unknown", "bitrig", "", "little", "64"),
            "x86_64-unknown-openbsd"=> ti("x86_64", "unknown", "openbsd", "", "little", "64"),
            "x86_64-unknown-netbsd"=> ti("x86_64", "unknown", "netbsd", "", "little", "64"),
            "x86_64-rumprun-netbsd"=> ti("x86_64", "rumprun", "netbsd", "", "little", "64"),
            "x86_64-apple-darwin"=> ti("x86_64", "apple", "macos", "", "little", "64"),
            "i686-apple-darwin"=> ti("x86", "apple", "macos", "", "little", "32"),
            "i386-apple-ios"=> ti("x86", "apple", "ios", "", "little", "32"),
            "x86_64-apple-ios"=> ti("x86_64", "apple", "ios", "", "little", "64"),
            "aarch64-apple-ios"=> ti("aarch64", "apple", "ios", "", "little", "64"),
            "armv7s-apple-ios" |
            "armv7-apple-ios"=> ti("arm", "apple", "ios", "", "little", "32"),
            "x86_64-sun-solaris"=> ti("x86_64", "sun", "solaris", "", "little", "64"),
            "x86_64-pc-windows-gnu"=> ti("x86_64", "pc", "windows", "gnu", "little", "64"),
            "i686-pc-windows-gnu"=> ti("x86", "pc", "windows", "gnu", "little", "32"),
            "x86_64-pc-windows-msvc"=> ti("x86_64", "pc", "windows", "msvc", "little", "64"),
            "i586-pc-windows-msvc" |
            "i686-pc-windows-msvc"=> ti("x86", "pc", "windows", "msvc", "little", "32"),
            "le32-unknown-nacl"=> ti("le32", "unknown", "nacl", "newlib", "little", "32"),
            "asmjs-unknown-emscripten"=> ti("asmjs", "unknown", "emscripten", "", "little", "32"),
            _ => None
        }
    }
}

impl TargetInfo {
    /// Architecture of the targeted machine
    ///
    /// Corresponds to the `#[cfg(target_arch)]` in Rust code.
    pub fn target_arch(&self) -> &str {
        &*self.arch
    }
    /// Vendor of the targeted machine
    ///
    /// Corresponds to the `#[cfg(target_vendor)]` in Rust code.
    pub fn target_vendor(&self) -> &str {
        &*self.vendor
    }
    /// OS of the targeted machine
    ///
    /// Corresponds to the `#[cfg(target_os)]` in Rust code.
    pub fn target_os(&self) -> &str {
        &*self.os
    }
    /// Environment (ABI) of the targeted machine
    ///
    /// Corresponds to the `#[cfg(target_env)]` in Rust code.
    pub fn target_env(&self) -> &str {
        &*self.env
    }
    /// Endianess of the targeted machine
    ///
    /// Valid values are: `little` and `big`.
    ///
    /// Corresponds to the `#[cfg(target_endian)]` in Rust code.
    pub fn target_endian(&self) -> &str {
        &*self.endian
    }
    /// Pointer width of the targeted machine
    ///
    /// Corresponds to the `#[cfg(target_pointer_width)]` in Rust code.
    pub fn target_pointer_width(&self) -> &str {
        &*self.pointer_width
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn correct_archs() {
        macro_rules! check_arch {
            ($expected: expr, $bit: expr, $end: expr, $($str: expr),+) => {
                $(
                    assert_eq!(super::TargetInfo::from_str($str).unwrap().target_arch(), $expected);
                    assert_eq!(super::TargetInfo::from_str($str).unwrap().target_endian(), $end);
                    assert_eq!(super::TargetInfo::from_str($str).unwrap().target_pointer_width(),
                               $bit);
                )+
            }
        }
        check_arch!("x86_64", "64", "little"
                   , "x86_64-unknown-linux-gnu"
                   , "x86_64-unknown-linux-musl"
                   , "x86_64-unknown-freebsd"
                   , "x86_64-unknown-dragonfly"
                   , "x86_64-unknown-bitrig"
                   , "x86_64-unknown-openbsd"
                   , "x86_64-unknown-netbsd"
                   , "x86_64-rumprun-netbsd"
                   , "x86_64-apple-darwin"
                   , "x86_64-apple-ios"
                   , "x86_64-sun-solaris"
                   , "x86_64-pc-windows-gnu"
                   , "x86_64-pc-windows-msvc");

        check_arch!("x86", "32", "little"
                   , "i586-unknown-linux-gnu"
                   , "i686-unknown-linux-musl"
                   , "i686-linux-android"
                   , "i686-unknown-freebsd"
                   , "i686-unknown-dragonfly"
                   , "i686-apple-darwin"
                   , "i686-pc-windows-gnu"
                   , "i686-pc-windows-msvc"
                   , "i586-pc-windows-msvc"
                   , "i386-apple-ios");
        check_arch!("mips", "32", "big"
                   , "mips-unknown-linux-musl"
                   , "mips-unknown-linux-gnu");
        check_arch!("mips", "32", "little"
                   , "mipsel-unknown-linux-musl"
                   , "mipsel-unknown-linux-gnu");
        check_arch!("aarch64", "64", "little"
                   , "aarch64-unknown-linux-gnu"
                   , "aarch64-linux-android"
                   , "aarch64-apple-ios");
        check_arch!("arm", "32", "little"
                   , "arm-unknown-linux-gnueabi"
                   , "arm-unknown-linux-gnueabihf"
                   , "arm-linux-androideabi"
                   , "armv7-linux-androideabi"
                   , "armv7-apple-ios");
        check_arch!("powerpc", "32", "big", "powerpc-unknown-linux-gnu");
        check_arch!("powerpc64", "64", "big"
                   , "powerpc64-unknown-linux-gnu");
        check_arch!("powerpc64", "64", "little"
                   , "powerpc64le-unknown-linux-gnu");
        check_arch!("le32", "32", "little", "le32-unknown-nacl");
        check_arch!("asmjs", "32", "little", "asmjs-unknown-emscripten");
    }

    #[test]
    fn correct_vendors() {
        macro_rules! check_vnd {
            ($expected: expr, $($str: expr),+) => {
                $(
                    assert_eq!(super::TargetInfo::from_str($str).unwrap().target_vendor(),
                               $expected);
                )+
            }
        }
        check_vnd!("unknown", "x86_64-unknown-linux-gnu"
                            , "x86_64-unknown-linux-musl"
                            , "x86_64-unknown-freebsd"
                            , "x86_64-unknown-dragonfly"
                            , "x86_64-unknown-bitrig"
                            , "x86_64-unknown-openbsd"
                            , "x86_64-unknown-netbsd"
                            , "i686-unknown-linux-gnu"
                            , "i586-unknown-linux-gnu"
                            , "i686-unknown-linux-musl"
                            , "i686-unknown-freebsd"
                            , "i686-unknown-dragonfly"
                            , "mips-unknown-linux-musl"
                            , "mips-unknown-linux-gnu"
                            , "mipsel-unknown-linux-musl"
                            , "mipsel-unknown-linux-gnu"
                            , "aarch64-unknown-linux-gnu"
                            , "arm-unknown-linux-gnueabi"
                            , "arm-unknown-linux-gnueabihf"
                            , "armv7-unknown-linux-gnueabihf"
                            , "le32-unknown-nacl"
                            , "asmjs-unknown-emscripten"
                            , "powerpc-unknown-linux-gnu"
                            , "powerpc64-unknown-linux-gnu"
                            , "powerpc64le-unknown-linux-gnu"
                            , "i686-linux-android"
                            , "aarch64-linux-android"
                            , "arm-linux-androideabi"
                            , "armv7-linux-androideabi");
        check_vnd!("apple", "x86_64-apple-darwin"
                          , "x86_64-apple-ios"
                          , "i686-apple-darwin"
                          , "i386-apple-ios"
                          , "aarch64-apple-ios"
                          , "armv7-apple-ios"
                          , "armv7s-apple-ios");
        check_vnd!("pc", "x86_64-pc-windows-gnu"
                       , "x86_64-pc-windows-msvc"
                       , "i686-pc-windows-gnu"
                       , "i686-pc-windows-msvc"
                       , "i586-pc-windows-msvc");
        check_vnd!("rumprun", "x86_64-rumprun-netbsd");
        check_vnd!("sun", "x86_64-sun-solaris");
    }

    #[test]
    fn correct_os() {
        macro_rules! check_os {
            ($expected: expr, $($str: expr),+) => {
                $(
                    assert_eq!(super::TargetInfo::from_str($str).unwrap().target_os(), $expected);
                )+
            }
        }
        check_os!("linux", "x86_64-unknown-linux-gnu"
                         , "x86_64-unknown-linux-musl"
                         , "i686-unknown-linux-gnu"
                         , "i586-unknown-linux-gnu"
                         , "i686-unknown-linux-musl"
                         , "mips-unknown-linux-musl"
                         , "mips-unknown-linux-gnu"
                         , "mipsel-unknown-linux-musl"
                         , "mipsel-unknown-linux-gnu"
                         , "aarch64-unknown-linux-gnu"
                         , "arm-unknown-linux-gnueabi"
                         , "arm-unknown-linux-gnueabihf"
                         , "armv7-unknown-linux-gnueabihf"
                         , "powerpc-unknown-linux-gnu"
                         , "powerpc64-unknown-linux-gnu"
                         , "powerpc64le-unknown-linux-gnu");
        check_os!("android", "i686-linux-android"
                           , "aarch64-linux-android"
                           , "arm-linux-androideabi"
                           , "armv7-linux-androideabi");
        check_os!("windows", "x86_64-pc-windows-gnu"
                           , "x86_64-pc-windows-msvc"
                           , "i686-pc-windows-gnu"
                           , "i686-pc-windows-msvc"
                           , "i586-pc-windows-msvc");
        check_os!("freebsd", "x86_64-unknown-freebsd"
                           , "i686-unknown-freebsd");
        check_os!("dragonfly", "x86_64-unknown-dragonfly"
                             , "i686-unknown-dragonfly");
        check_os!("bitrig", "x86_64-unknown-bitrig");
        check_os!("openbsd", "x86_64-unknown-openbsd");
        check_os!("netbsd", "x86_64-unknown-netbsd"
                          , "x86_64-rumprun-netbsd");
        check_os!("solaris", "x86_64-sun-solaris");
        check_os!("macos", "x86_64-apple-darwin"
                         , "i686-apple-darwin");
        check_os!("ios", "x86_64-apple-ios"
                       , "i386-apple-ios"
                       , "aarch64-apple-ios"
                       , "armv7-apple-ios"
                       , "armv7s-apple-ios");
        check_os!("nacl", "le32-unknown-nacl");
        check_os!("emscripten", "asmjs-unknown-emscripten");
    }

    #[test]
    fn correct_env() {
        macro_rules! check_env {
            ($expected: expr, $($str: expr),+) => {
                $(
                    assert_eq!(super::TargetInfo::from_str($str).unwrap().target_env(), $expected);
                )+
            }
        }
        check_env!("gnu", "x86_64-unknown-linux-gnu"
                        , "i686-unknown-linux-gnu"
                        , "i586-unknown-linux-gnu"
                        , "mips-unknown-linux-gnu"
                        , "mipsel-unknown-linux-gnu"
                        , "aarch64-unknown-linux-gnu"
                        , "arm-unknown-linux-gnueabi"
                        , "arm-unknown-linux-gnueabihf"
                        , "armv7-unknown-linux-gnueabihf"
                        , "powerpc-unknown-linux-gnu"
                        , "powerpc64-unknown-linux-gnu"
                        , "powerpc64le-unknown-linux-gnu"
                        , "x86_64-pc-windows-gnu"
                        , "i686-pc-windows-gnu");
        check_env!("musl", "x86_64-unknown-linux-musl"
                         , "i686-unknown-linux-musl"
                         , "mips-unknown-linux-musl"
                         , "mipsel-unknown-linux-musl");
        check_env!("msvc", "x86_64-pc-windows-msvc"
                         , "i686-pc-windows-msvc"
                         , "i586-pc-windows-msvc");
        check_env!("", "i686-linux-android"
                     , "aarch64-linux-android"
                     , "arm-linux-androideabi"
                     , "armv7-linux-androideabi"
                     , "x86_64-unknown-freebsd"
                     , "i686-unknown-freebsd"
                     , "x86_64-unknown-dragonfly"
                     , "i686-unknown-dragonfly"
                     , "x86_64-unknown-bitrig"
                     , "x86_64-unknown-openbsd"
                     , "x86_64-unknown-netbsd"
                     , "x86_64-rumprun-netbsd"
                     , "x86_64-sun-solaris"
                     , "x86_64-apple-darwin"
                     , "i686-apple-darwin"
                     , "x86_64-apple-ios"
                     , "i386-apple-ios"
                     , "aarch64-apple-ios"
                     , "armv7-apple-ios"
                     , "armv7s-apple-ios"
                     , "asmjs-unknown-emscripten");
        check_env!("newlib", "le32-unknown-nacl");
    }

    #[test]
    fn external_work() {
        use std::env;
        env::set_var("TARGET", "src/my-great-target.json");
        let target = super::TargetInfo::new().unwrap();
        external_is_correct(&target);
    }

    #[test]
    fn external_search_work() {
        use std::env;
        env::set_var("RUST_TARGET_PATH", "");
        super::TargetInfo::from_str("my-great-target").err().unwrap();
        env::set_var("RUST_TARGET_PATH", "/usr/");
        super::TargetInfo::from_str("my-great-target").err().unwrap();
        env::set_var("RUST_TARGET_PATH", "/usr/:src/");
        let target = super::TargetInfo::from_str("my-great-target").unwrap();
        external_is_correct(&target);
    }

    fn external_is_correct(ti: &super::TargetInfo) {
        assert_eq!(ti.target_arch(), "x86_64");
        assert_eq!(ti.target_endian(), "little");
        assert_eq!(ti.target_pointer_width(), "42");
        assert_eq!(ti.target_os(), "nux");
        assert_eq!(ti.target_vendor(), "unknown");
        assert_eq!(ti.target_env(), "");
    }
}
