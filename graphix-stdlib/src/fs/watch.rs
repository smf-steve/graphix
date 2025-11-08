use crate::deftype;
use anyhow::{anyhow, bail};
use arcstr::{literal, ArcStr};
use compact_str::{format_compact, CompactString};
use enumflags2::{bitflags, BitFlags};
use futures::{channel::mpsc, future::join_all, SinkExt, StreamExt};
use fxhash::{FxHashMap, FxHashSet};
use graphix_compiler::{
    errf, expr::ExprId, Apply, BindId, BuiltIn, BuiltInInitFn, CustomBuiltinType, Event,
    ExecCtx, LibState, Node, Rt, UserEvent, CBATCH_POOL,
};
use netidx::utils::Either;
use netidx_value::{FromValue, ValArray, Value};
use notify::{
    event::{
        AccessKind, CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind,
        RenameMode,
    },
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use poolshark::{
    global::{GPooled, Pool},
    local::LPooled,
};
use std::{
    any::Any,
    collections::{hash_set, VecDeque},
    ffi::OsString,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    result::Result,
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::{fs, select, sync::mpsc as tmpsc, task};

static PATHS: LazyLock<Pool<FxHashSet<ArcStr>>> = LazyLock::new(|| Pool::new(1000, 1000));

#[derive(Debug, Clone, Copy)]
#[bitflags]
#[repr(u64)]
enum Interest {
    Any,
    Access,
    AccessOpen,
    AccessClose,
    AccessRead,
    AccessOther,
    Create,
    CreateFile,
    CreateFolder,
    CreateOther,
    Modify,
    ModifyData,
    ModifyDataSize,
    ModifyDataContent,
    ModifyDataOther,
    ModifyMetadata,
    ModifyMetadataAccessTime,
    ModifyMetadataWriteTime,
    ModifyMetadataPermissions,
    ModifyMetadataOwnership,
    ModifyMetadataExtended,
    ModifyMetadataOther,
    ModifyRename,
    ModifyRenameTo,
    ModifyRenameFrom,
    ModifyRenameBoth,
    ModifyRenameOther,
    ModifyOther,
    Delete,
    DeleteFile,
    DeleteFolder,
    DeleteOther,
    Other,
}

macro_rules! impl_value_conv {
    ($enum:ident { $($variant:ident),* $(,)? }) => {
        impl FromValue for $enum {
            fn from_value(v: Value) -> anyhow::Result<Self> {
                match v {
                    Value::String(s) => match &*s {
                        $(stringify!($variant) => Ok(Self::$variant),)*
                        _ => Err(anyhow::anyhow!("Invalid {} variant: {}", stringify!($enum), s)),
                    },
                    _ => Err(anyhow::anyhow!("Expected string value for {}, got: {:?}", stringify!($enum), v)),
                }
            }
        }

        impl Into<Value> for $enum {
            fn into(self) -> Value {
                match self {
                    $(Self::$variant => Value::String(literal!(stringify!($variant))),)*
                }
            }
        }
    };
}

impl_value_conv!(Interest {
    Any,
    Access,
    AccessOpen,
    AccessClose,
    AccessRead,
    AccessOther,
    Create,
    CreateFile,
    CreateFolder,
    CreateOther,
    Modify,
    ModifyData,
    ModifyDataSize,
    ModifyDataContent,
    ModifyDataOther,
    ModifyMetadata,
    ModifyMetadataAccessTime,
    ModifyMetadataWriteTime,
    ModifyMetadataPermissions,
    ModifyMetadataOwnership,
    ModifyMetadataExtended,
    ModifyMetadataOther,
    ModifyRename,
    ModifyRenameTo,
    ModifyRenameFrom,
    ModifyRenameBoth,
    ModifyRenameOther,
    ModifyOther,
    Delete,
    DeleteFile,
    DeleteFolder,
    DeleteOther,
    Other,
});

impl From<&EventKind> for Interest {
    fn from(kind: &EventKind) -> Self {
        match kind {
            EventKind::Any => Self::Any,
            EventKind::Access(AccessKind::Any) => Self::Access,
            EventKind::Access(AccessKind::Close(_)) => Self::AccessClose,
            EventKind::Access(AccessKind::Open(_)) => Self::AccessOpen,
            EventKind::Access(AccessKind::Read) => Self::AccessRead,
            EventKind::Access(AccessKind::Other) => Self::AccessOther,
            EventKind::Create(CreateKind::Any) => Self::Create,
            EventKind::Create(CreateKind::File) => Self::CreateFile,
            EventKind::Create(CreateKind::Folder) => Self::CreateFolder,
            EventKind::Create(CreateKind::Other) => Self::CreateOther,
            EventKind::Modify(ModifyKind::Any) => Self::Modify,
            EventKind::Modify(ModifyKind::Data(DataChange::Any)) => Self::ModifyData,
            EventKind::Modify(ModifyKind::Data(DataChange::Content)) => {
                Self::ModifyDataContent
            }
            EventKind::Modify(ModifyKind::Data(DataChange::Size)) => Self::ModifyDataSize,
            EventKind::Modify(ModifyKind::Data(DataChange::Other)) => {
                Self::ModifyDataOther
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any)) => {
                Self::ModifyMetadata
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)) => {
                Self::ModifyMetadataAccessTime
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Extended)) => {
                Self::ModifyMetadataExtended
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Other)) => {
                Self::ModifyMetadataOther
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Ownership)) => {
                Self::ModifyMetadataOwnership
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions)) => {
                Self::ModifyMetadataPermissions
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)) => {
                Self::ModifyMetadataWriteTime
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => Self::ModifyRename,
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                Self::ModifyRenameBoth
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                Self::ModifyRenameFrom
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => Self::ModifyRenameTo,
            EventKind::Modify(ModifyKind::Name(RenameMode::Other)) => {
                Self::ModifyRenameOther
            }
            EventKind::Modify(ModifyKind::Other) => Self::ModifyOther,
            EventKind::Remove(RemoveKind::Any) => Self::Delete,
            EventKind::Remove(RemoveKind::File) => Self::DeleteFile,
            EventKind::Remove(RemoveKind::Folder) => Self::DeleteFolder,
            EventKind::Remove(RemoveKind::Other) => Self::DeleteOther,
            EventKind::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone)]
struct Watch {
    path: ArcStr,
    id: BindId,
    interest: BitFlags<Interest>,
}

impl Watch {
    fn interested(&self, kind: &EventKind) -> bool {
        use Interest::*;
        match kind {
            EventKind::Any => !self.interest.is_empty(),
            EventKind::Access(AccessKind::Any) => self
                .interest
                .intersects(Access | AccessClose | AccessOpen | AccessRead | AccessOther),
            EventKind::Access(AccessKind::Close(_)) => {
                self.interest.intersects(Access | AccessClose)
            }
            EventKind::Access(AccessKind::Open(_)) => {
                self.interest.intersects(Access | AccessOpen)
            }
            EventKind::Access(AccessKind::Read) => {
                self.interest.intersects(Access | AccessRead)
            }
            EventKind::Access(AccessKind::Other) => {
                self.interest.intersects(Access | AccessOther)
            }
            EventKind::Create(CreateKind::Any) => {
                self.interest.intersects(Create | CreateFile | CreateFolder | CreateOther)
            }
            EventKind::Create(CreateKind::File) => {
                self.interest.intersects(Create | CreateFile)
            }
            EventKind::Create(CreateKind::Folder) => {
                self.interest.intersects(Create | CreateFolder)
            }
            EventKind::Create(CreateKind::Other) => {
                self.interest.intersects(Create | CreateOther)
            }
            EventKind::Modify(ModifyKind::Any) => self.interest.intersects(
                Modify
                    | ModifyData
                    | ModifyDataSize
                    | ModifyDataContent
                    | ModifyDataOther
                    | ModifyMetadata
                    | ModifyMetadataAccessTime
                    | ModifyMetadataWriteTime
                    | ModifyMetadataPermissions
                    | ModifyMetadataOwnership
                    | ModifyMetadataExtended
                    | ModifyMetadataOther
                    | ModifyRename
                    | ModifyRenameTo
                    | ModifyRenameFrom
                    | ModifyRenameBoth
                    | ModifyRenameOther
                    | ModifyOther,
            ),
            EventKind::Modify(ModifyKind::Data(DataChange::Any)) => {
                self.interest.intersects(
                    Modify
                        | ModifyData
                        | ModifyDataSize
                        | ModifyDataContent
                        | ModifyDataOther,
                )
            }
            EventKind::Modify(ModifyKind::Data(DataChange::Content)) => {
                self.interest.intersects(Modify | ModifyData | ModifyDataContent)
            }
            EventKind::Modify(ModifyKind::Data(DataChange::Size)) => {
                self.interest.intersects(Modify | ModifyData | ModifyDataSize)
            }
            EventKind::Modify(ModifyKind::Data(DataChange::Other)) => {
                self.interest.intersects(Modify | ModifyData | ModifyDataOther)
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any)) => {
                self.interest.intersects(
                    Modify
                        | ModifyMetadata
                        | ModifyMetadataAccessTime
                        | ModifyMetadataWriteTime
                        | ModifyMetadataPermissions
                        | ModifyMetadataOwnership
                        | ModifyMetadataExtended
                        | ModifyMetadataOther,
                )
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)) => self
                .interest
                .intersects(Modify | ModifyMetadata | ModifyMetadataAccessTime),
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Extended)) => {
                self.interest.intersects(Modify | ModifyMetadata | ModifyMetadataExtended)
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Other)) => {
                self.interest.intersects(Modify | ModifyMetadata | ModifyMetadataOther)
            }
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Ownership)) => self
                .interest
                .intersects(Modify | ModifyMetadata | ModifyMetadataOwnership),
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions)) => self
                .interest
                .intersects(Modify | ModifyMetadata | ModifyMetadataPermissions),
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)) => self
                .interest
                .intersects(Modify | ModifyMetadata | ModifyMetadataWriteTime),
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
                self.interest.intersects(
                    Modify
                        | ModifyRename
                        | ModifyRenameTo
                        | ModifyRenameFrom
                        | ModifyRenameBoth
                        | ModifyRenameOther,
                )
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                self.interest.intersects(Modify | ModifyRename | ModifyRenameBoth)
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                self.interest.intersects(Modify | ModifyRename | ModifyRenameFrom)
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                self.interest.intersects(Modify | ModifyRename | ModifyRenameTo)
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::Other)) => {
                self.interest.intersects(Modify | ModifyRename | ModifyRenameOther)
            }
            EventKind::Modify(ModifyKind::Other) => {
                self.interest.intersects(Modify | ModifyOther)
            }
            EventKind::Remove(RemoveKind::Any) => {
                self.interest.intersects(Delete | DeleteFile | DeleteFolder | DeleteOther)
            }
            EventKind::Remove(RemoveKind::File) => {
                self.interest.intersects(Delete | DeleteFile)
            }
            EventKind::Remove(RemoveKind::Folder) => {
                self.interest.intersects(Delete | DeleteFolder)
            }
            EventKind::Remove(RemoveKind::Other) => {
                self.interest.intersects(Delete | DeleteOther)
            }
            EventKind::Other => self.interest.contains(Other),
        }
    }
}

#[derive(Debug)]
enum WatchCmd {
    Watch(Watch),
    Stop(BindId),
}

#[derive(Debug, Clone)]
enum WatchEventKind {
    Established,
    Error(ArcStr),
    Event(Interest),
}

#[derive(Debug)]
struct WatchEvent {
    paths: GPooled<FxHashSet<ArcStr>>,
    event: WatchEventKind,
}

impl CustomBuiltinType for WatchEvent {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathStatus {
    /// the canonical part of the path that exists
    exists: PathBuf,
    /// the path parts that are missing, in reverse order
    missing: LPooled<Vec<OsString>>,
    /// the full path including the missing parts, which aren't canonical
    full_path: PathBuf,
}

impl PathStatus {
    fn established(&self) -> bool {
        self.missing.is_empty()
    }

    async fn new(path: &Path) -> Self {
        let mut missing: LPooled<Vec<OsString>> = LPooled::take();
        let mut root: &Path = &path;
        let mut t = loop {
            match fs::canonicalize(root).await {
                Ok(exists) => {
                    break Self { exists: exists.clone(), missing, full_path: exists }
                }
                Err(_) => match (root.parent(), root.file_name()) {
                    (None, None) => {
                        break Self {
                            exists: PathBuf::from(path),
                            missing,
                            full_path: PathBuf::from(path),
                        }
                    }
                    (None, Some(_)) => {
                        break Self {
                            exists: PathBuf::from(root),
                            missing,
                            full_path: PathBuf::from(root),
                        }
                    }
                    (Some(parent), None) => root = parent,
                    (Some(parent), Some(file)) => {
                        missing.push(OsString::from(file));
                        root = parent;
                    }
                },
            }
        };
        for part in t.missing.iter().rev() {
            t.full_path.push(part);
        }
        t
    }
}

fn utf8_path(path: &PathBuf) -> ArcStr {
    let path = path.as_os_str().as_bytes();
    match str::from_utf8(path) {
        Ok(s) => ArcStr::from(s),
        Err(_) => ArcStr::from(CompactString::from_utf8_lossy(path).as_str()),
    }
}

#[derive(Debug, Clone)]
struct WatchInt {
    watch: Watch,
    path_status: PathStatus,
}

enum AddAction {
    AddWatch(PathBuf),
    JustNotify(PathBuf),
    AddPending { watch_path: PathBuf, full_path: PathBuf },
}

struct ChangeOfStatus {
    remove: Option<PathBuf>,
    add: Option<AddAction>,
}

#[derive(Default)]
struct Watched {
    by_id: FxHashMap<BindId, WatchInt>,
    pending: FxHashMap<BindId, WatchInt>,
    by_root: FxHashMap<PathBuf, FxHashSet<BindId>>,
}

impl Watched {
    /// add a watch and return the necessary action to the notify::Watcher
    async fn add_watch(&mut self, w: Watch) -> AddAction {
        let id = w.id;
        let path_status =
            match self.by_id.get_mut(&id).or_else(|| self.pending.get_mut(&id)) {
                Some(ow) if ow.watch.path == w.path => {
                    let WatchInt {
                        watch: Watch { path: _, id: _, interest },
                        path_status,
                    } = ow;
                    *interest = w.interest;
                    return AddAction::JustNotify(path_status.full_path.clone());
                }
                Some(_) | None => PathStatus::new(&Path::new(&*w.path)).await,
            };
        let w = WatchInt { watch: w, path_status };
        let watch_path = w.path_status.exists.clone();
        let full_path = w.path_status.full_path.clone();
        let established = w.path_status.established();
        self.by_root.entry(watch_path.clone()).or_default().insert(id);
        if established {
            self.by_id.insert(id, w);
            AddAction::AddWatch(watch_path)
        } else {
            self.pending.insert(id, w);
            AddAction::AddPending { watch_path, full_path }
        }
    }

    /// remove a watch, and return an optional action to be performed on the
    /// watcher
    fn remove_watch(&mut self, id: &BindId) -> (Option<WatchInt>, Option<PathBuf>) {
        let w = self.by_id.remove(id).or_else(|| self.pending.remove(id));
        let to_stop =
            w.as_ref().and_then(|w| match self.by_root.get_mut(&w.path_status.exists) {
                None => Some(w.path_status.exists.clone()),
                Some(ids) => {
                    ids.remove(id);
                    ids.is_empty().then(|| {
                        self.by_root.remove(&w.path_status.exists);
                        w.path_status.exists.clone()
                    })
                }
            });
        (w, to_stop)
    }

    /// inform of a change of status for this watch id, return a set of
    /// necessary actions to the notify::Watcher
    async fn change_status(&mut self, id: BindId) -> ChangeOfStatus {
        let (w, remove) = self.remove_watch(&id);
        let add = match w {
            Some(w) => Some(self.add_watch(w.watch).await),
            None => None,
        };
        let (remove, add) = match (&remove, &add) {
            (Some(remove), Some(AddAction::AddPending { watch_path, full_path: _ }))
            | (Some(remove), Some(AddAction::AddWatch(watch_path)))
                if remove == watch_path =>
            {
                (None, None)
            }
            (Some(_), Some(AddAction::AddPending { .. }))
            | (Some(_), Some(AddAction::AddWatch(_)))
            | (Some(_), Some(AddAction::JustNotify(_)))
            | (None, Some(_))
            | (Some(_), None)
            | (None, None) => (remove, add),
        };
        ChangeOfStatus { remove, add }
    }

    fn relevant_to<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a WatchInt> {
        struct I<'a> {
            level: usize,
            ids: LPooled<VecDeque<hash_set::Iter<'a, BindId>>>,
            t: &'a Watched,
        }
        impl<'a> Iterator for I<'a> {
            type Item = &'a WatchInt;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    match self.ids.front_mut() {
                        None => break None,
                        Some(set) => match set.next() {
                            Some(id) => match self.t.by_id.get(id) {
                                Some(w) if self.level < 2 => break Some(w),
                                Some(_) => continue,
                                None => match self.t.pending.get(id) {
                                    Some(w) => break Some(w),
                                    None => continue,
                                },
                            },
                            None => {
                                self.level += 1;
                                self.ids.pop_front();
                            }
                        },
                    }
                }
            }
        }
        let mut ids: LPooled<VecDeque<hash_set::Iter<'a, BindId>>> = LPooled::take();
        let mut root = Some(path);
        while let Some(path) = root {
            if let Some(h) = self.by_root.get(path) {
                ids.push_back(h.iter())
            }
            root = path.parent();
        }
        I { level: 0, ids, t: self }
    }

    /// poll all the pending watches and return a list of ids who's status might have changed
    async fn poll_pending(&self) -> LPooled<Vec<BindId>> {
        join_all(self.pending.iter().map(|(id, w)| async {
            if tokio::fs::try_exists(&*w.watch.path).await.unwrap_or(false) {
                Some(*id)
            } else {
                None
            }
        }))
        .await
        .into_iter()
        .filter_map(|x| x)
        .collect()
    }

    async fn process_event(
        &mut self,
        batch: &mut GPooled<Vec<(BindId, Box<dyn CustomBuiltinType>)>>,
        ev: Result<notify::Event, notify::Error>,
    ) -> LPooled<Vec<BindId>> {
        let mut by_id: LPooled<FxHashMap<BindId, WatchEvent>> = LPooled::take();
        let mut status_changed: LPooled<Vec<BindId>> = LPooled::take();
        match ev {
            Ok(ev) => {
                let event = WatchEventKind::Event((&ev.kind).into());
                for path in ev.paths.iter() {
                    let utf8_path = utf8_path(path);
                    for w in self.relevant_to(path) {
                        macro_rules! report {
                            () => {{
                                let wev = by_id.entry(w.watch.id).or_insert_with(|| {
                                    WatchEvent {
                                        event: event.clone(),
                                        paths: PATHS.take(),
                                    }
                                });
                                wev.paths.insert(utf8_path.clone());
                            }};
                        }
                        if w.path_status.established() {
                            if w.watch.interested(&ev.kind) {
                                report!();
                            }
                            if let EventKind::Remove(_) = &ev.kind
                                && path == &w.path_status.exists
                            {
                                status_changed.push(w.watch.id)
                            }
                        } else {
                            if let EventKind::Create(_) = &ev.kind {
                                if w.watch.interested(&ev.kind)
                                    && tokio::fs::try_exists(&*w.watch.path)
                                        .await
                                        .unwrap_or(false)
                                {
                                    report!()
                                }
                                status_changed.push(w.watch.id);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let err = ArcStr::from(format_compact!("{:?}", e.kind).as_str());
                for path in e.paths.iter() {
                    let utf8_path = utf8_path(path);
                    for w in self.relevant_to(&path) {
                        match &e.kind {
                            notify::ErrorKind::PathNotFound => {
                                status_changed.push(w.watch.id)
                            }
                            _ => {
                                let wev = by_id.entry(w.watch.id).or_insert_with(|| {
                                    WatchEvent {
                                        paths: PATHS.take(),
                                        event: WatchEventKind::Error(err.clone()),
                                    }
                                });
                                wev.paths.insert(utf8_path.clone());
                            }
                        }
                    }
                }
            }
        }
        let evs = by_id
            .drain()
            .map(|(id, wev)| (id, Box::new(wev) as Box<dyn CustomBuiltinType>));
        batch.extend(evs);
        status_changed
    }
}

fn push_error(
    batch: &mut GPooled<Vec<(BindId, Box<dyn CustomBuiltinType>)>>,
    id: BindId,
    path: Option<ArcStr>,
    e: Either<notify::Error, ArcStr>,
) {
    let mut wev = WatchEvent {
        paths: PATHS.take(),
        event: WatchEventKind::Error(match e {
            Either::Left(e) => ArcStr::from(format_compact!("{e:?}").as_str()),
            Either::Right(e) => e,
        }),
    };
    if let Some(path) = path {
        wev.paths.insert(path);
    }
    batch.push((id, Box::new(wev)))
}

fn push_established(
    batch: &mut GPooled<Vec<(BindId, Box<dyn CustomBuiltinType>)>>,
    id: BindId,
    path: &PathBuf,
) {
    let path = utf8_path(path);
    let mut wev = WatchEvent { paths: PATHS.take(), event: WatchEventKind::Established };
    wev.paths.insert(path);
    batch.push((id, Box::new(wev)))
}

async fn file_watcher_loop(
    mut watcher: RecommendedWatcher,
    mut rx_notify: tmpsc::Receiver<notify::Result<notify::Event>>,
    mut rx: mpsc::UnboundedReceiver<WatchCmd>,
    mut tx: mpsc::Sender<GPooled<Vec<(BindId, Box<dyn CustomBuiltinType>)>>>,
) {
    let mut watched = Watched::default();
    let mut recv_buf = vec![];
    let mut batch = CBATCH_POOL.take();
    let mut poll_interval = tokio::time::interval(Duration::from_secs(1));
    macro_rules! or_push {
        ($path:expr, $id:expr, $r:expr) => {
            if let Err(e) = $r {
                let path = utf8_path(&$path);
                push_error(&mut batch, $id, Some(path), Either::Left(e))
            }
        };
    }
    macro_rules! add_watch {
        ($path:expr, $id:expr, on_success: $success:block) => {
            match watcher.watch($path, RecursiveMode::NonRecursive) {
                Ok(()) => $success,
                Err(e) => {
                    push_error(&mut batch, $id, Some(utf8_path($path)), Either::Left(e))
                }
            }
        };
    }
    macro_rules! status_change {
        ($id:expr) => {{
            let stc = watched.change_status($id).await;
            if let Some(path) = stc.remove {
                or_push!(&path, $id, watcher.unwatch(&path));
            }
            match stc.add {
                None | Some(AddAction::JustNotify(_)) => (),
                Some(AddAction::AddWatch(watch_path)) | Some(AddAction::AddPending { watch_path, full_path: _}) => {
                    add_watch!(&watch_path, $id, on_success: { () })
                }
            }
        }}
    }
    loop {
        select! {
            _ = poll_interval.tick() => {
                for id in watched.poll_pending().await.drain(..) {
                    status_change!(id)
                }
            },
            n = rx_notify.recv_many(&mut recv_buf, 10000) => {
                if n == 0 {
                    break
                }
                for ev in recv_buf.drain(..) {
                    let mut status = watched.process_event(&mut batch, ev).await;
                    for id in status.drain(..) {
                        status_change!(id)
                    }
                }
            },
            cmd = rx.next() => match cmd {
                None => break,
                Some(WatchCmd::Watch(w)) => {
                    let id = w.id;
                    match watched.add_watch(w).await {
                        AddAction::JustNotify(path) => push_established(&mut batch, id, &path),
                        AddAction::AddWatch(path) => {
                            add_watch!(&path, id, on_success: {
                                push_established(&mut batch, id, &path)
                            })
                        }
                        AddAction::AddPending { watch_path, full_path } => {
                            add_watch!(&watch_path, id, on_success: {
                                push_established(&mut batch, id, &full_path);
                            })
                        }
                    }
                },
                Some(WatchCmd::Stop(id)) => match watched.remove_watch(&id).1 {
                    None => (),
                    Some(path) => {
                        or_push!(path, id, watcher.unwatch(&path))
                    }
                }
            },
        }
        if !batch.is_empty() {
            if let Err(_) = tx.send(batch).await {
                break;
            }
            batch = CBATCH_POOL.take()
        }
    }
    let mut batch = CBATCH_POOL.take();
    while let Ok(Some(cmd)) = rx.try_next() {
        match cmd {
            WatchCmd::Stop(_) => (),
            WatchCmd::Watch(w) => {
                let e = literal!("the watcher thread has stopped");
                push_error(&mut batch, w.id, None, Either::Right(e));
            }
        }
    }
    if !batch.is_empty() {
        let _ = tx.send(batch).await;
    }
}

struct WatchCtx(anyhow::Result<mpsc::UnboundedSender<WatchCmd>>);

impl WatchCtx {
    fn send(&self, cmd: WatchCmd) -> anyhow::Result<()> {
        match &self.0 {
            Ok(tx) => tx.unbounded_send(cmd).map_err(|_| anyhow!("watcher died")),
            Err(e) => bail!("could not start watcher {e:?}"),
        }
    }
}

struct NotifyChan(tmpsc::Sender<notify::Result<notify::Event>>);

impl notify::EventHandler for NotifyChan {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let _ = self.0.blocking_send(event);
    }
}

fn get_watcher<'a, R: Rt>(rt: &mut R, st: &'a mut LibState) -> &'a mut WatchCtx {
    st.get_or_else::<WatchCtx, _>(|| {
        let (notify_tx, notify_rx) = tmpsc::channel(10);
        let notify_tx = NotifyChan(notify_tx);
        match notify::recommended_watcher(notify_tx) {
            Err(e) => WatchCtx(Err(e.into())),
            Ok(watcher) => {
                let (cmd_tx, cmd_rx) = mpsc::unbounded();
                let (ev_tx, ev_rx) = mpsc::channel(1000);
                task::spawn(async move {
                    file_watcher_loop(watcher, notify_rx, cmd_rx, ev_tx).await
                });
                rt.watch(ev_rx);
                WatchCtx(Ok(cmd_tx))
            }
        }
    })
}

macro_rules! watch {
    (
        type_name: $type_name:ident,
        builtin_name: $builtin_name:literal,
        graphix_type: $graphix_type:literal,
        handle_event: |$id:ident, $ctx:ident, $ev:ident| $handle_event:block
    ) => {
        #[derive(Debug)]
        pub(super) struct $type_name {
            id: BindId,
            top_id: ExprId,
            interest: Option<BitFlags<Interest>>,
            path: Option<ArcStr>,
        }

        impl<R: Rt, E: UserEvent> BuiltIn<R, E> for $type_name {
            const NAME: &str = $builtin_name;
            deftype!("fs", $graphix_type);

            fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
                Arc::new(|ctx, _, _, _, top_id| {
                    let id = BindId::new();
                    ctx.rt.ref_var(id, top_id);
                    Ok(Box::new($type_name { id, top_id, interest: None, path: None }))
                })
            }
        }

        impl<R: Rt, E: UserEvent> Apply<R, E> for $type_name {
            fn update(
                &mut self,
                ctx: &mut ExecCtx<R, E>,
                from: &mut [Node<R, E>],
                event: &mut Event<E>,
            ) -> Option<Value> {
                let mut up = false;
                if let Some(Ok(mut int)) = from[0]
                    .update(ctx, event)
                    .map(|v| v.cast_to::<LPooled<Vec<Interest>>>())
                {
                    let int = int.drain(..).fold(BitFlags::empty(), |mut acc, fl| {
                        acc.insert(fl);
                        acc
                    });
                    up |= self.interest != Some(int);
                    self.interest = Some(int);
                }
                if let Some(Ok(path)) =
                    from[1].update(ctx, event).map(|v| v.cast_to::<ArcStr>())
                {
                    let path = Some(path);
                    up = path != self.path;
                    self.path = path;
                }
                if up
                    && let Some(path) = &self.path
                    && let Some(interest) = self.interest
                {
                    let wctx = get_watcher(&mut ctx.rt, &mut ctx.libstate);
                    if let Err(e) = wctx.send(WatchCmd::Watch(Watch {
                        path: path.clone(),
                        id: self.id,
                        interest,
                    })) {
                        ctx.rt.set_var(self.id, errf!("WatchError", "{e:?}"));
                    }
                }
                if let Some(mut w) = event.custom.remove(&self.id)
                    && let Some(w) =
                        (&mut *w as &mut dyn Any).downcast_mut::<WatchEvent>()
                {
                    let $id = self.id;
                    let $ctx = ctx;
                    let $ev = w;
                    $handle_event;
                }
                event.variables.get(&self.id).cloned()
            }

            fn sleep(&mut self, ctx: &mut ExecCtx<R, E>) {
                let wctx = get_watcher(&mut ctx.rt, &mut ctx.libstate);
                let _ = wctx.send(WatchCmd::Stop(self.id));
                ctx.rt.unref_var(self.id, self.top_id);
                self.id = BindId::new();
                ctx.rt.ref_var(self.id, self.top_id);
            }

            fn delete(&mut self, ctx: &mut ExecCtx<R, E>) {
                let wctx = get_watcher(&mut ctx.rt, &mut ctx.libstate);
                let _ = wctx.send(WatchCmd::Stop(self.id));
                ctx.rt.unref_var(self.id, self.top_id);
            }
        }
    };
}

watch!(
    type_name: WatchBuiltIn,
    builtin_name: "fs_watch",
    graphix_type: "fn(?#interest:Array<Interest>, string) -> Result<string, `WatchError(string)>",
    handle_event: |id, ctx, w| {
        match &w.event {
            WatchEventKind::Established | WatchEventKind::Event(_) => {
                for p in w.paths.drain() {
                    ctx.rt.set_var(id, Value::String(p))
                }
            }
            WatchEventKind::Error(e) => ctx.rt.set_var(id, errf!("WatchError", "{e:?}")),
        }
    }
);

watch!(
    type_name: WatchFullBuiltIn,
    builtin_name: "fs_watch_full",
    graphix_type: "fn(?#interest:Array<Interest>, string) -> Result<WatchEvent, `WatchError(string)>",
    handle_event: |id, ctx, w| {
        let paths =
            Value::Array(ValArray::from_iter_exact(w.paths.drain().map(Value::String)));
        match &w.event {
            WatchEventKind::Error(e) => ctx.rt.set_var(id, errf!("WatchError", "{e:?}")),
            WatchEventKind::Established => {
                let e = (
                    (literal!("event"), literal!("Established")),
                    (literal!("paths"), paths),
                );
                ctx.rt.set_var(id, e.into())
            }
            WatchEventKind::Event(int) => {
                let e =
                    ((literal!("event"), Value::from(int)), (literal!("paths"), paths));
                ctx.rt.set_var(id, e.into())
            }
        }
    }
);
