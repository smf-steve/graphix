use crate::{
    deftype, CachedArgs, CachedArgsAsync, CachedVals, EvalCached, EvalCachedAsync,
};
use anyhow::Result;
use arcstr::{literal, ArcStr};
use chrono::{DateTime, Utc};
use compact_str::CompactString;
use graphix_compiler::{errf, ExecCtx, Rt, UserEvent};
use netidx_value::Value;
use parking_lot::Mutex;
use poolshark::local::LPooled;
use std::{fmt, path::PathBuf, sync::Arc};
use tempfile::TempDir;

mod file;
mod watch;

#[derive(Debug)]
enum Name {
    Prefix(ArcStr),
    Suffix(ArcStr),
}

struct TempDirArgs {
    dir: Option<ArcStr>,
    name: Option<Name>,
    t: Arc<Mutex<Option<TempDir>>>,
}

impl fmt::Debug for TempDirArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ dir: {:?}, name: {:?} }}", self.dir, self.name)
    }
}

#[derive(Debug, Default)]
struct GxTempDirEv {
    current: Arc<Mutex<Option<TempDir>>>,
}

impl EvalCachedAsync for GxTempDirEv {
    const NAME: &str = "fs_tempdir";
    deftype!(
        "fs",
        r#"fn(?#in:[null, string],
              ?#name:[null, `Prefix(string), `Suffix(string)],
              Any)
           -> Result<string, `IOError(string)>"#
    );
    type Args = TempDirArgs;

    fn prepare_args(&mut self, cached: &CachedVals) -> Option<Self::Args> {
        if cached.0.iter().any(|v| v.is_none()) {
            None
        } else {
            let dir = cached.get::<Option<ArcStr>>(0).flatten();
            let name = cached
                .get::<Option<(ArcStr, ArcStr)>>(1)
                .and_then(|v| v)
                .and_then(|(tag, v)| match &*tag {
                    "Prefix" => Some(Name::Prefix(v)),
                    "Suffix" => Some(Name::Suffix(v)),
                    _ => None,
                });
            let _ = cached.get::<Value>(2)?;
            Some(TempDirArgs { dir, name, t: self.current.clone() })
        }
    }

    fn eval(args: Self::Args) -> impl Future<Output = Value> + Send {
        async move {
            let td = tokio::task::spawn_blocking(|| match (args.dir, args.name) {
                (None, None) => TempDir::new(),
                (None, Some(Name::Prefix(pfx))) => TempDir::with_prefix(&*pfx),
                (None, Some(Name::Suffix(sfx))) => TempDir::with_suffix(&*sfx),
                (Some(dir), None) => TempDir::new_in(&*dir),
                (Some(dir), Some(Name::Prefix(pfx))) => {
                    TempDir::with_prefix_in(&*pfx, &*dir)
                }
                (Some(dir), Some(Name::Suffix(sfx))) => {
                    TempDir::with_suffix_in(&*sfx, &*dir)
                }
            })
            .await;
            match td {
                Err(e) => errf!("IOError", "failed to spawn create temp dir {e:?}"),
                Ok(Err(e)) => errf!("IOError", "failed to create temp dir {e:?}"),
                Ok(Ok(td)) => {
                    use std::fmt::Write;
                    let mut buf = CompactString::new("");
                    write!(buf, "{}", td.path().display()).unwrap();
                    *args.t.lock() = Some(td);
                    Value::String(ArcStr::from(buf.as_str()))
                }
            }
        }
    }
}

type GxTempDir = CachedArgsAsync<GxTempDirEv>;

#[derive(Debug, Default)]
struct IsFileEv;

impl EvalCachedAsync for IsFileEv {
    const NAME: &str = "fs_is_file";
    deftype!("fs", "fn(string) -> Result<string, `IOError(string)>");
    type Args = ArcStr;

    fn prepare_args(&mut self, cached: &CachedVals) -> Option<Self::Args> {
        cached.get::<ArcStr>(0)
    }

    fn eval(path: Self::Args) -> impl Future<Output = Value> + Send {
        async move {
            match tokio::fs::metadata(&*path).await {
                Err(e) => errf!("IOError", "can't stat {e:?}"),
                Ok(m) if m.is_file() => Value::String(path),
                Ok(_) => errf!("IOError", "not a file"),
            }
        }
    }
}

type IsFile = CachedArgsAsync<IsFileEv>;

#[derive(Debug, Default)]
struct IsDirEv;

impl EvalCachedAsync for IsDirEv {
    const NAME: &str = "fs_is_dir";
    deftype!("fs", "fn(string) -> Result<string, `IOError(string)>");
    type Args = ArcStr;

    fn prepare_args(&mut self, cached: &CachedVals) -> Option<Self::Args> {
        cached.get::<ArcStr>(0)
    }

    fn eval(path: Self::Args) -> impl Future<Output = Value> + Send {
        async move {
            match tokio::fs::metadata(&*path).await {
                Err(e) => errf!("IOError", "can't stat {e:?}"),
                Ok(m) if m.is_dir() => Value::String(path),
                Ok(_) => errf!("IOError", "not a directory"),
            }
        }
    }
}

type IsDir = CachedArgsAsync<IsDirEv>;

#[derive(Debug, Default)]
struct MetadataEv;

impl EvalCachedAsync for MetadataEv {
    const NAME: &str = "fs_metadata";
    deftype!(
        "fs",
        "fn(?#follow_symlinks:bool, string) -> Result<Metadata, `IOError(string)>"
    );
    type Args = (bool, ArcStr);

    fn prepare_args(&mut self, cached: &CachedVals) -> Option<Self::Args> {
        Some((cached.get::<bool>(0)?, cached.get::<ArcStr>(1)?))
    }

    fn eval((follow_symlinks, path): Self::Args) -> impl Future<Output = Value> + Send {
        async move {
            let md = if follow_symlinks {
                tokio::fs::metadata(&*path).await
            } else {
                tokio::fs::symlink_metadata(&*path).await
            };
            match md {
                Err(e) => errf!("IOError", "could not stat {e:?}"),
                Ok(m) => {
                    let accessed: Option<DateTime<Utc>> =
                        m.accessed().ok().map(|ts| ts.into());
                    let created: Option<DateTime<Utc>> =
                        m.created().ok().map(|ts| ts.into());
                    let modified: Option<DateTime<Utc>> =
                        m.modified().ok().map(|ts| ts.into());
                    let kind = {
                        let typ = m.file_type();
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::FileTypeExt;
                            if typ.is_dir() {
                                Value::String(literal!("Dir"))
                            } else if typ.is_file() {
                                Value::String(literal!("File"))
                            } else if typ.is_symlink() {
                                Value::String(literal!("Symlink"))
                            } else if typ.is_block_device() {
                                Value::String(literal!("BlockDev"))
                            } else if typ.is_char_device() {
                                Value::String(literal!("CharDev"))
                            } else if typ.is_fifo() {
                                Value::String(literal!("Fifo"))
                            } else if typ.is_socket() {
                                Value::String(literal!("Socket"))
                            } else {
                                Value::Null
                            }
                        }
                        #[cfg(windows)]
                        {
                            use std::os::windows::fs::FileTypeExt;
                            if typ.is_dir() {
                                Value::String(literal!("Dir"))
                            } else if typ.is_file() {
                                Value::String(literal!("File"))
                            } else if typ.is_symlink_file() {
                                Value::String(literal!("Symlink"))
                            } else if typ.is_symlink_dir() {
                                Value::String(literal!("SymlinkDir"))
                            } else {
                                Value::Null
                            }
                        }
                    };
                    let len = m.len();
                    let permissions = {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::MetadataExt;
                            Value::U32(m.mode())
                        }
                        #[cfg(windows)]
                        {
                            Value::from((
                                literal!("ReadOnly"),
                                m.permissions().readonly(),
                            ))
                        }
                    };
                    let r: [(ArcStr, Value); 6] = [
                        (literal!("accessed"), accessed.into()),
                        (literal!("created"), created.into()),
                        (literal!("kind"), kind),
                        (literal!("len"), len.into()),
                        (literal!("modified"), modified.into()),
                        (literal!("permissions"), permissions),
                    ];
                    r.into()
                }
            }
        }
    }
}

type Metadata = CachedArgsAsync<MetadataEv>;

#[derive(Debug, Default)]
struct JoinPathEv;

impl EvalCached for JoinPathEv {
    const NAME: &str = "fs_join_path";
    deftype!("fs", "fn(string, @args: [string, Array<string>]) -> string");

    fn eval(&mut self, from: &CachedVals) -> Option<Value> {
        let mut parts: LPooled<Vec<ArcStr>> = LPooled::take();
        for part in from.0.iter() {
            match part {
                None => return None,
                Some(Value::String(s)) => parts.push(s.clone()),
                Some(Value::Array(a)) => {
                    for part in a.iter() {
                        match part {
                            Value::String(s) => parts.push(s.clone()),
                            _ => return None,
                        }
                    }
                }
                _ => return None,
            }
        }
        let mut path = PathBuf::new();
        for part in parts.drain(..) {
            path.push(&*part)
        }
        let mut buf = CompactString::new("");
        use std::fmt::Write;
        write!(buf, "{}", path.display()).unwrap();
        Some(Value::String(ArcStr::from(buf.as_str())))
    }
}

type JoinPath = CachedArgs<JoinPathEv>;

pub(super) fn register<R: Rt, E: UserEvent>(ctx: &mut ExecCtx<R, E>) -> Result<ArcStr> {
    ctx.register_builtin::<GxTempDir>()?;
    ctx.register_builtin::<JoinPath>()?;
    ctx.register_builtin::<IsFile>()?;
    ctx.register_builtin::<IsDir>()?;
    ctx.register_builtin::<Metadata>()?;
    ctx.register_builtin::<watch::SetGlobals>()?;
    ctx.register_builtin::<watch::WatchBuiltIn>()?;
    ctx.register_builtin::<watch::WatchFullBuiltIn>()?;
    ctx.register_builtin::<file::ReadAll>()?;
    ctx.register_builtin::<file::ReadAllBin>()?;
    ctx.register_builtin::<file::WriteAll>()?;
    ctx.register_builtin::<file::WriteAllBin>()?;
    Ok(literal!(include_str!("fs.gx")))
}
