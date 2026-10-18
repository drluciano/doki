//! JNI bridge: `dev.doki.bridge.Doki.call(vaultPath, requestJson, opener)`.
//!
//! Java supplies save folders through the `dev.doki.bridge.SaveFolder`
//! interface (on Android, backed by the Storage Access Framework). This
//! crate adapts that interface to [`SaveLocation`] and forwards the JSON
//! request to [`doki_api::handle`]. All safety logic stays in the engine:
//! Java only reads, stages, replaces and removes the files it is told to.
//!
//! Java exceptions map to engine errors: `java.io.FileNotFoundException` →
//! `SourceUnavailable`, `java.lang.SecurityException` → `PermissionLost`,
//! anything else → `Io`. Panics never cross the boundary; they become an
//! `Internal` error response.

use std::cell::RefCell;
use std::io::{self, Read};
use std::panic::{self, AssertUnwindSafe};

use doki_core::error::StorageSide;
use doki_core::{BundlePath, Error, Result};
use doki_engine::LocationProvider;
use doki_platform::{Capabilities, ReplaceMode, SaveLocation, Support};
use jni::objects::{JByteArray, JClass, JObject, JObjectArray, JString, JValue};
use jni::sys::jstring;
use jni::JNIEnv;

const SAVE_FOLDER: &str = "Ldev/doki/bridge/SaveFolder;";

/// Turn a pending Java exception (or JNI failure) into an engine error.
fn java_err(env: &mut JNIEnv, e: jni::errors::Error, what: &str) -> Error {
    if !matches!(e, jni::errors::Error::JavaException) {
        return Error::Io { context: what.to_owned(), source: io::Error::other(e.to_string()) };
    }
    let Ok(throwable) = env.exception_occurred() else {
        return Error::Io { context: what.to_owned(), source: io::Error::other("Java exception") };
    };
    let _ = env.exception_clear();
    let describe = |env: &mut JNIEnv, obj: &JObject, method: &str| -> String {
        env.call_method(obj, method, "()Ljava/lang/String;", &[])
            .and_then(|v| v.l())
            .and_then(|s| env.get_string(&JString::from(s)).map(|s| s.into()))
            .unwrap_or_default()
    };
    let class = env
        .call_method(&throwable, "getClass", "()Ljava/lang/Class;", &[])
        .and_then(|v| v.l())
        .map(|c| describe(env, &c, "getName"))
        .unwrap_or_default();
    let message = describe(env, &throwable, "toString");
    let _ = env.exception_clear();
    match class.as_str() {
        "java.io.FileNotFoundException" => Error::SourceUnavailable(message),
        "java.lang.SecurityException" => Error::PermissionLost(message),
        _ => Error::io(what, StorageSide::SaveLocation, io::Error::other(message)),
    }
}

/// Opens save folders by calling `SaveFolderOpener.open(location)`.
struct JniProvider<'a, 'l> {
    env: &'a RefCell<JNIEnv<'l>>,
    opener: &'a JObject<'l>,
}

impl<'a, 'l> LocationProvider for JniProvider<'a, 'l> {
    fn open<'b>(&'b self, location: &str) -> Box<dyn SaveLocation + 'b> {
        let env = &mut *self.env.borrow_mut();
        let result = (|| {
            let loc = env.new_string(location)?;
            let folder = env
                .call_method(
                    self.opener,
                    "open",
                    format!("(Ljava/lang/String;){SAVE_FOLDER}"),
                    &[JValue::Object(&loc)],
                )?
                .l()?;
            env.new_global_ref(folder)
        })();
        match result {
            Ok(folder) => Box::new(JniLocation { env: self.env, folder }),
            Err(e) => Box::new(match java_err(env, e, "opening save folder") {
                Error::PermissionLost(m) => Unavailable { permission_lost: true, message: m },
                Error::SourceUnavailable(m) => Unavailable { permission_lost: false, message: m },
                other => Unavailable { permission_lost: false, message: other.to_string() },
            }),
        }
    }
}

/// A folder Java could not open; every operation reports why.
struct Unavailable {
    permission_lost: bool,
    message: String,
}

impl Unavailable {
    fn err(&self) -> Error {
        if self.permission_lost {
            Error::PermissionLost(self.message.clone())
        } else {
            Error::SourceUnavailable(self.message.clone())
        }
    }
}

impl SaveLocation for Unavailable {
    fn describe(&self) -> String {
        self.message.clone()
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities { replace: ReplaceMode::Unknown, durable_flush: Support::Unknown }
    }
    fn check_available(&self) -> Result<()> {
        Err(self.err())
    }
    fn list(&self) -> Result<Vec<BundlePath>> {
        Err(self.err())
    }
    fn open(&self, _: &BundlePath) -> Result<Box<dyn Read + '_>> {
        Err(self.err())
    }
    fn stage(&self, _: &str, _: &BundlePath, _: &mut dyn Read) -> Result<()> {
        Err(self.err())
    }
    fn open_staged(&self, _: &str, _: &BundlePath) -> Result<Box<dyn Read + '_>> {
        Err(self.err())
    }
    fn commit_staged(&self, _: &str, _: &BundlePath) -> Result<()> {
        Err(self.err())
    }
    fn remove(&self, _: &BundlePath) -> Result<()> {
        Err(self.err())
    }
    fn discard_staging(&self, _: &str) -> Result<()> {
        Err(self.err())
    }
}

struct JniLocation<'a, 'l> {
    env: &'a RefCell<JNIEnv<'l>>,
    folder: jni::objects::GlobalRef,
}

impl JniLocation<'_, '_> {
    /// Call a method in its own local-reference frame, mapping exceptions.
    fn call<T>(
        &self,
        what: &str,
        f: impl FnOnce(&mut JNIEnv, &JObject) -> jni::errors::Result<T>,
    ) -> Result<T> {
        let env = &mut *self.env.borrow_mut();
        let folder = self.folder.as_obj();
        let result = env.with_local_frame(16, |env| f(env, folder));
        result.map_err(|e| java_err(env, e, what))
    }

    /// Call a `String[]` method taking no argument or one string.
    fn strings(&self, method: &str, arg: Option<&str>) -> Result<Vec<String>> {
        self.call("listing save folder", |env, f| {
            let value = match arg {
                Some(a) => {
                    let a = env.new_string(a)?;
                    env.call_method(
                        f,
                        method,
                        "(Ljava/lang/String;)[Ljava/lang/String;",
                        &[JValue::Object(&a)],
                    )?
                }
                None => env.call_method(f, method, "()[Ljava/lang/String;", &[])?,
            };
            let arr = JObjectArray::from(value.l()?);
            let n = env.get_array_length(&arr)?;
            let mut out = Vec::with_capacity(n as usize);
            for i in 0..n {
                // Free each element: Android caps local references per frame.
                let s = JString::from(env.get_object_array_element(&arr, i)?);
                out.push(env.get_string(&s)?.into());
                env.delete_local_ref(s)?;
            }
            Ok(out)
        })
    }

    fn bytes(env: &mut JNIEnv, value: JObject) -> jni::errors::Result<Vec<u8>> {
        env.convert_byte_array(JByteArray::from(value))
    }
}

impl SaveLocation for JniLocation<'_, '_> {
    fn describe(&self) -> String {
        self.call("describing save folder", |env, f| {
            let s = env.call_method(f, "describe", "()Ljava/lang/String;", &[])?.l()?;
            env.get_string(&JString::from(s)).map(Into::into)
        })
        .unwrap_or_else(|e| e.to_string())
    }

    fn capabilities(&self) -> Capabilities {
        let mode = self
            .call("checking save folder", |env, f| {
                env.call_method(f, "replaceMode", "()I", &[])?.i()
            })
            .unwrap_or(0);
        Capabilities {
            replace: match mode {
                1 => ReplaceMode::Atomic,
                2 => ReplaceMode::DeleteThenRename,
                _ => ReplaceMode::Unknown,
            },
            durable_flush: Support::Unknown,
        }
    }

    fn check_available(&self) -> Result<()> {
        self.call("checking save folder", |env, f| {
            env.call_method(f, "checkAvailable", "()V", &[]).map(|_| ())
        })
    }

    fn list(&self) -> Result<Vec<BundlePath>> {
        let names = self.strings("list", None)?;
        let mut paths = names.iter().map(|n| BundlePath::new(n)).collect::<Result<Vec<_>>>()?;
        paths.sort();
        Ok(paths)
    }

    fn list_under(&self, dir: &BundlePath) -> Result<Vec<BundlePath>> {
        let names = self.strings("listUnder", Some(dir.as_str()))?;
        let mut paths = names.iter().map(|n| BundlePath::new(n)).collect::<Result<Vec<_>>>()?;
        paths.sort();
        Ok(paths)
    }

    fn entries(&self) -> Result<Vec<String>> {
        let mut names = self.strings("entries", None)?;
        names.sort();
        Ok(names)
    }

    fn open(&self, path: &BundlePath) -> Result<Box<dyn Read + '_>> {
        let bytes = self.call(&format!("reading {path}"), |env, f| {
            let p = env.new_string(path.as_str())?;
            let v =
                env.call_method(f, "read", "(Ljava/lang/String;)[B", &[JValue::Object(&p)])?.l()?;
            Self::bytes(env, v)
        })?;
        Ok(Box::new(io::Cursor::new(bytes)))
    }

    fn stage(&self, op: &str, path: &BundlePath, data: &mut dyn Read) -> Result<()> {
        let mut buf = Vec::new();
        data.read_to_end(&mut buf)
            .map_err(|e| Error::io("reading data to stage", StorageSide::Vault, e))?;
        self.call(&format!("staging {path}"), |env, f| {
            let (o, p, d) = (
                env.new_string(op)?,
                env.new_string(path.as_str())?,
                env.byte_array_from_slice(&buf)?,
            );
            env.call_method(
                f,
                "stage",
                "(Ljava/lang/String;Ljava/lang/String;[B)V",
                &[JValue::Object(&o), JValue::Object(&p), JValue::Object(&d)],
            )
            .map(|_| ())
        })
    }

    fn open_staged(&self, op: &str, path: &BundlePath) -> Result<Box<dyn Read + '_>> {
        let bytes = self.call(&format!("reading staged {path}"), |env, f| {
            let (o, p) = (env.new_string(op)?, env.new_string(path.as_str())?);
            let v = env
                .call_method(
                    f,
                    "readStaged",
                    "(Ljava/lang/String;Ljava/lang/String;)[B",
                    &[JValue::Object(&o), JValue::Object(&p)],
                )?
                .l()?;
            Self::bytes(env, v)
        })?;
        Ok(Box::new(io::Cursor::new(bytes)))
    }

    fn commit_staged(&self, op: &str, path: &BundlePath) -> Result<()> {
        self.call(&format!("replacing {path}"), |env, f| {
            let (o, p) = (env.new_string(op)?, env.new_string(path.as_str())?);
            env.call_method(
                f,
                "commitStaged",
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[JValue::Object(&o), JValue::Object(&p)],
            )
            .map(|_| ())
        })
    }

    fn remove(&self, path: &BundlePath) -> Result<()> {
        self.call(&format!("removing {path}"), |env, f| {
            let p = env.new_string(path.as_str())?;
            env.call_method(f, "remove", "(Ljava/lang/String;)V", &[JValue::Object(&p)]).map(|_| ())
        })
    }

    fn discard_staging(&self, op: &str) -> Result<()> {
        self.call("removing staged files", |env, f| {
            let o = env.new_string(op)?;
            env.call_method(f, "discardStaging", "(Ljava/lang/String;)V", &[JValue::Object(&o)])
                .map(|_| ())
        })
    }
}

fn internal_error(msg: &str) -> String {
    format!(
        r#"{{"ok":false,"error":{{"code":"Internal","message":{},"save_may_be_mixed":false}}}}"#,
        serde_json_string(msg)
    )
}

fn serde_json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `static native String call(String vaultPath, String requestJson, SaveFolderOpener opener)`
#[no_mangle]
pub extern "system" fn Java_dev_doki_bridge_Doki_call<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    vault: JString<'l>,
    request: JString<'l>,
    opener: JObject<'l>,
) -> jstring {
    let inputs: jni::errors::Result<(String, String)> =
        (|| Ok((env.get_string(&vault)?.into(), env.get_string(&request)?.into())))();
    let response = match inputs {
        Err(e) => internal_error(&format!("bad arguments: {e}")),
        Ok((vault, request)) => {
            let cell = RefCell::new(unsafe { env.unsafe_clone() });
            let provider = JniProvider { env: &cell, opener: &opener };
            panic::catch_unwind(AssertUnwindSafe(|| doki_api::handle(&vault, provider, &request)))
                .unwrap_or_else(|_| internal_error("the engine stopped unexpectedly"))
        }
    };
    match env.new_string(response) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
