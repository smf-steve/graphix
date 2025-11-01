use arcstr::{literal, ArcStr};
use compact_str::format_compact;
use enumflags2::{bitflags, BitFlags};
use futures::{channel::mpsc, SinkExt, StreamExt};
use fxhash::{FxHashMap, FxHashSet};
use graphix_compiler::{
    err, errf, BindId, CustomBuiltinType, LibState, Rt, UserEvent, CBATCH_POOL,
};
use netidx_value::Value;
use notify::{
    event::{
        AccessKind, CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind,
        RenameMode,
    },
    EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use poolshark::global::GPooled;
use std::{
    collections::hash_set,
    path::{Path, PathBuf},
    pin::Pin,
    result::Result,
};
use tokio::{fs, select, sync::mpsc as tmpsc, task};

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

impl Into<Value> for Interest {
    fn into(self) -> Value {
        use Interest::*;
        match self {
            Any => Value::String(literal!("Any")),
            Access => Value::String(literal!("Access")),
            AccessOpen => Value::String(literal!("AccessOpen")),
            AccessClose => Value::String(literal!("AccessClose")),
            AccessRead => Value::String(literal!("AccessRead")),
            AccessOther => Value::String(literal!("AccessOther")),
            Create => Value::String(literal!("Create")),
            CreateFile => Value::String(literal!("CreateFile")),
            CreateFolder => Value::String(literal!("CreateFolder")),
            CreateOther => Value::String(literal!("CreateOther")),
            Modify => Value::String(literal!("Modify")),
            ModifyData => Value::String(literal!("ModifyData")),
            ModifyDataSize => Value::String(literal!("ModifyDataSize")),
            ModifyDataContent => Value::String(literal!("ModifyDataContent")),
            ModifyDataOther => Value::String(literal!("ModifyDataOther")),
            ModifyMetadata => Value::String(literal!("ModifyMetadata")),
            ModifyMetadataAccessTime => {
                Value::String(literal!("ModifyMetadataAccessTime"))
            }
            ModifyMetadataWriteTime => Value::String(literal!("ModifyMetadataWriteTime")),
            ModifyMetadataPermissions => {
                Value::String(literal!("ModifyMetadataPermissions"))
            }
            ModifyMetadataOwnership => Value::String(literal!("ModifyMetadataOwnership")),
            ModifyMetadataExtended => Value::String(literal!("ModifyMetadataExtended")),
            ModifyMetadataOther => Value::String(literal!("ModifyMetadataOther")),
            ModifyRename => Value::String(literal!("ModifyRename")),
            ModifyRenameTo => Value::String(literal!("ModifyRenameTo")),
            ModifyRenameFrom => Value::String(literal!("ModifyRenameFrom")),
            ModifyRenameBoth => Value::String(literal!("ModifyRenameBoth")),
            ModifyRenameOther => Value::String(literal!("ModifyRenameOther")),
            ModifyOther => Value::String(literal!("ModifyOther")),
            Delete => Value::String(literal!("Delete")),
            DeleteFile => Value::String(literal!("DeleteFile")),
            DeleteFolder => Value::String(literal!("DeleteFolder")),
            DeleteOther => Value::String(literal!("DeleteOther")),
            Other => Value::String(literal!("Other")),
        }
    }
}

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
    canonical_path: PathBuf,
    id: BindId,
    interest: BitFlags<Interest>,
    recursive: bool,
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

enum WatchCmd {
    Watch(Watch),
    Stop(BindId),
}

#[derive(Debug)]
enum WatchEvent {
    Update { path: PathBuf, event: Interest },
    Error { path: Option<PathBuf>, error: ArcStr },
}

impl CustomBuiltinType for WatchEvent {}

/// like fs::cananocialize, but will never fail. It will canonicalize
/// as much of the path as it's possible to canonicalize and leave the
/// rest untouched.
fn best_effort_canonicalize(
    path: PathBuf,
) -> Pin<Box<dyn Future<Output = PathBuf> + Send + Sync + 'static>> {
    Box::pin(async move {
        match fs::canonicalize(&path).await {
            Ok(p) => p,
            Err(_) => match (path.parent(), path.file_name()) {
                (None, None) => PathBuf::new(),
                (None, Some(_)) => PathBuf::from(path),
                (Some(parent), None) => best_effort_canonicalize(parent.into()).await,
                (Some(parent), Some(file)) => {
                    best_effort_canonicalize(parent.into()).await.join(file)
                }
            },
        }
    })
}

#[derive(Default)]
struct Watched {
    by_id: FxHashMap<BindId, Watch>,
    by_root: FxHashMap<PathBuf, FxHashSet<BindId>>,
}

impl Watched {
    /// return an iterator over all the watches that are relevant to a particular path
    fn relevant_to<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a Watch> {
        struct I<'a> {
            t: &'a Watched,
            path: Option<&'a Path>,
            curr: Option<hash_set::Iter<'a, BindId>>,
        }
        impl<'a> Iterator for I<'a> {
            type Item = &'a Watch;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    if let Some(sl) = self.curr.as_mut()
                        && let Some(id) = sl.next()
                    {
                        match self.t.by_id.get(id) {
                            Some(w) => break Some(w),
                            None => continue,
                        }
                    }
                    match self.path {
                        None => break None,
                        Some(path) => match self.t.by_root.get(path) {
                            None => self.path = path.parent(),
                            Some(ids) => {
                                self.path = path.parent();
                                self.curr = Some(ids.iter())
                            }
                        },
                    }
                }
            }
        }
        I { t: self, path: Some(path), curr: None }
    }

    /// add a watch and return the canonicalized path if adding a watch is necessary
    async fn add_watch(&mut self, mut w: Watch) -> Option<PathBuf> {
        let id = w.id;
        let path = best_effort_canonicalize(PathBuf::from(&*w.path)).await;
        w.canonical_path = path.clone();
        self.by_id.insert(id, w);
        let s = self.by_root.entry(path.clone()).or_default();
        let add = s.is_empty();
        s.insert(id);
        add.then(|| path)
    }

    /// remove a watch, and return it's root path if it is the last
    /// watch with that root
    fn remove_watch(&mut self, id: &BindId) -> Option<PathBuf> {
        self.by_id.remove(id).and_then(|w| {
            match self.by_root.get_mut(&w.canonical_path) {
                None => Some(w.canonical_path),
                Some(ids) => {
                    ids.remove(id);
                    if ids.is_empty() {
                        self.by_root.remove(&w.canonical_path);
                        Some(w.canonical_path)
                    } else {
                        None
                    }
                }
            }
        })
    }

    fn process_event(
        &mut self,
        batch: &mut GPooled<Vec<(BindId, Box<dyn CustomBuiltinType>)>>,
        ev: Result<notify::Event, notify::Error>,
    ) {
        match ev {
            Ok(ev) => {
                for path in ev.paths.iter() {
                    for w in self.relevant_to(&path) {
                        if w.interested(&ev.kind) {
                            let wev = WatchEvent::Update {
                                path: path.clone(),
                                event: (&ev.kind).into(),
                            };
                            batch.push((w.id, Box::new(wev)))
                        }
                    }
                }
            }
            Err(e) => {
                let err = ArcStr::from(format_compact!("{:?}", e.kind).as_str());
                for path in e.paths.iter() {
                    for w in self.relevant_to(&path) {
                        let wev = WatchEvent::Error {
                            path: Some(path.clone()),
                            error: err.clone(),
                        };
                        batch.push((w.id, Box::new(wev)))
                    }
                }
            }
        }
    }
}

fn push_error(
    batch: &mut GPooled<Vec<(BindId, Box<dyn CustomBuiltinType>)>>,
    id: BindId,
    path: Option<PathBuf>,
    e: notify::Error,
) {
    let wev = WatchEvent::Error {
        path: path,
        error: ArcStr::from(format_compact!("{e:?}").as_str()),
    };
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
    loop {
        select! {
            n = rx_notify.recv_many(&mut recv_buf, 10000) => {
                if n == 0 {
                    break
                }
                for ev in recv_buf.drain(..) {
                    watched.process_event(&mut batch, ev)
                }
            },
            cmd = rx.select_next_some() => match cmd {
                WatchCmd::Watch(w) => {
                    let recursive = if w.recursive {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };
                    let id = w.id;
                    if let Some(path) = watched.add_watch(w).await {
                        if let Err(e) = watcher.watch(&path, recursive) {
                            push_error(&mut batch, id, Some(path), e)
                        }
                    }
                },
                WatchCmd::Stop(id) => {
                    if let Some(path) = watched.remove_watch(&id) {
                        if let Err(e) = watcher.unwatch(&path) {
                            push_error(&mut batch, id, Some(path), e)
                        }
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
                let wev = WatchEvent::Error {
                    path: None,
                    error: literal!("the watcher thread has stopped"),
                };
                batch.push((w.id, Box::new(wev)));
            }
        }
    }
    if !batch.is_empty() {
        let _ = tx.send(batch).await;
    }
}

struct WatchCtx(mpsc::UnboundedSender<WatchCmd>);

struct NotifyChan(tmpsc::Sender<notify::Result<notify::Event>>);

impl notify::EventHandler for NotifyChan {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let _ = self.0.blocking_send(event);
    }
}

fn get_watcher<'a, R: Rt, E: UserEvent>(
    rt: &mut R,
    st: &'a mut LibState,
) -> anyhow::Result<&'a mut WatchCtx> {
    if st.contains::<WatchCtx>() {
        Ok(st.get_mut::<WatchCtx>().unwrap())
    } else {
        let (notify_tx, notify_rx) = tmpsc::channel(10);
        let notify_tx = NotifyChan(notify_tx);
        let watcher = notify::recommended_watcher(notify_tx)?;
        let (cmd_tx, cmd_rx) = mpsc::unbounded();
        let (ev_tx, ev_rx) = mpsc::channel(1000);
        task::spawn(
            async move { file_watcher_loop(watcher, notify_rx, cmd_rx, ev_tx).await },
        );
        rt.watch(ev_rx);
        st.set::<WatchCtx>(WatchCtx(cmd_tx));
        Ok(st.get_mut::<WatchCtx>().unwrap())
    }
}
