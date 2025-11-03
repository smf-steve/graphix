use anyhow::{anyhow, bail};
use arcstr::{literal, ArcStr};
use compact_str::{format_compact, CompactString};
use enumflags2::{bitflags, BitFlags};
use futures::{channel::mpsc, SinkExt, StreamExt};
use fxhash::{FxHashMap, FxHashSet};
use graphix_compiler::{
    errf, expr::ExprId, Apply, BindId, BuiltIn, BuiltInInitFn, CustomBuiltinType, Event,
    ExecCtx, LibState, Node, Rt, UserEvent, CBATCH_POOL,
};
use netidx::utils::Either;
use netidx_value::{FromValue, Value};
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
    collections::{hash_set, HashSet},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    pin::Pin,
    result::{self, Result},
    sync::{Arc, LazyLock},
};
use tokio::{fs, select, sync::mpsc as tmpsc, task};

use crate::deftype;

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

impl FromValue for Interest {
    fn from_value(v: Value) -> anyhow::Result<Self> {
        match v {
            Value::String(s) => match &*s {
                "Any" => Ok(Self::Any),
                "Access" => Ok(Self::Access),
                "AccessOpen" => Ok(Self::AccessOpen),
                "AccessClose" => Ok(Self::AccessClose),
                "AccessRead" => Ok(Self::AccessRead),
                "AccessOther" => Ok(Self::AccessOther),
                "Create" => Ok(Self::Create),
                "CreateFile" => Ok(Self::CreateFile),
                "CreateFolder" => Ok(Self::CreateFolder),
                "CreateOther" => Ok(Self::CreateOther),
                "Modify" => Ok(Self::Modify),
                "ModifyData" => Ok(Self::ModifyData),
                "ModifyDataSize" => Ok(Self::ModifyDataSize),
                "ModifyDataContent" => Ok(Self::ModifyDataContent),
                "ModifyDataOther" => Ok(Self::ModifyDataOther),
                "ModifyMetadata" => Ok(Self::ModifyMetadata),
                "ModifyMetadataAccessTime" => Ok(Self::ModifyMetadataAccessTime),
                "ModifyMetadataWriteTime" => Ok(Self::ModifyMetadataWriteTime),
                "ModifyMetadataPermissions" => Ok(Self::ModifyMetadataPermissions),
                "ModifyMetadataOwnership" => Ok(Self::ModifyMetadataOwnership),
                "ModifyMetadataExtended" => Ok(Self::ModifyMetadataExtended),
                "ModifyMetadataOther" => Ok(Self::ModifyMetadataOther),
                "ModifyRename" => Ok(Self::ModifyRename),
                "ModifyRenameTo" => Ok(Self::ModifyRenameTo),
                "ModifyRenameFrom" => Ok(Self::ModifyRenameFrom),
                "ModifyRenameBoth" => Ok(Self::ModifyRenameBoth),
                "ModifyRenameOther" => Ok(Self::ModifyRenameOther),
                "ModifyOther" => Ok(Self::ModifyOther),
                "Delete" => Ok(Self::Delete),
                "DeleteFile" => Ok(Self::DeleteFile),
                "DeleteFolder" => Ok(Self::DeleteFolder),
                "DeleteOther" => Ok(Self::DeleteOther),
                "Other" => Ok(Self::Other),
                _ => Err(anyhow::anyhow!("Invalid Interest variant: {}", s)),
            },
            _ => Err(anyhow::anyhow!("Expected string value for Interest, got: {:?}", v)),
        }
    }
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
struct WatchEvent {
    paths: GPooled<FxHashSet<ArcStr>>,
    event: result::Result<Interest, ArcStr>,
}

impl CustomBuiltinType for WatchEvent {}

impl WatchEvent {
    fn downcast_mut(t: &mut Box<dyn CustomBuiltinType>) -> Option<&mut Self> {
        (t as &mut dyn Any).downcast_mut()
    }
}

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

fn utf8_path(path: &PathBuf) -> ArcStr {
    let path = path.as_os_str().as_bytes();
    match str::from_utf8(path) {
        Ok(s) => ArcStr::from(s),
        Err(_) => ArcStr::from(CompactString::from_utf8_lossy(path).as_str()),
    }
}

#[derive(Default)]
struct Watched {
    by_id: FxHashMap<BindId, Watch>,
    by_root: FxHashMap<PathBuf, (RecursiveMode, FxHashSet<BindId>)>,
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
                            Some((_, ids)) => {
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
    async fn add_watch(&mut self, mut w: Watch) -> Option<(PathBuf, RecursiveMode)> {
        let id = w.id;
        let path = best_effort_canonicalize(PathBuf::from(&*w.path)).await;
        w.canonical_path = path.clone();
        let rec = if w.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        self.by_id.insert(id, w);
        let (is_rec, s) =
            self.by_root.entry(path.clone()).or_insert_with(|| (rec, HashSet::default()));
        let add = s.is_empty() || &rec != is_rec;
        *is_rec = rec;
        s.insert(id);
        add.then(|| (path, rec))
    }

    /// remove a watch, and return it's root path if it is the last
    /// watch with that root
    fn remove_watch(&mut self, id: &BindId) -> Option<PathBuf> {
        self.by_id.remove(id).and_then(|w| {
            match self.by_root.get_mut(&w.canonical_path) {
                None => Some(w.canonical_path),
                Some((_, ids)) => {
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
        let mut by_id: LPooled<FxHashMap<BindId, WatchEvent>> = LPooled::take();
        match ev {
            Ok(ev) => {
                let event = Ok((&ev.kind).into());
                for path in ev.paths.iter() {
                    let utf8_path = utf8_path(path);
                    for w in self.relevant_to(&path) {
                        if w.interested(&ev.kind) {
                            let wev = by_id.entry(w.id).or_insert_with(|| WatchEvent {
                                event: event.clone(),
                                paths: PATHS.take(),
                            });
                            wev.paths.insert(utf8_path.clone());
                        }
                    }
                }
            }
            Err(e) => {
                let err = ArcStr::from(format_compact!("{:?}", e.kind).as_str());
                for path in e.paths.iter() {
                    let utf8_path = utf8_path(path);
                    for w in self.relevant_to(&path) {
                        let wev = by_id.entry(w.id).or_insert_with(|| WatchEvent {
                            paths: PATHS.take(),
                            event: Err(err.clone()),
                        });
                        wev.paths.insert(utf8_path.clone());
                    }
                }
            }
        }
        let evs = by_id
            .drain()
            .map(|(id, wev)| (id, Box::new(wev) as Box<dyn CustomBuiltinType>));
        batch.extend(evs)
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
        event: Err(match e {
            Either::Left(e) => ArcStr::from(format_compact!("{e:?}").as_str()),
            Either::Right(e) => e,
        }),
    };
    if let Some(path) = path {
        wev.paths.insert(path);
    }
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
                    let id = w.id;
                    if let Some((path, recursive)) = watched.add_watch(w).await {
                        if let Err(e) = watcher.watch(&path, recursive) {
                            let path = utf8_path(&path);
                            push_error(&mut batch, id, Some(path), Either::Left(e))
                        }
                    }
                },
                WatchCmd::Stop(id) => {
                    if let Some(path) = watched.remove_watch(&id) {
                        if let Err(e) = watcher.unwatch(&path) {
                            let path = utf8_path(&path);
                            push_error(&mut batch, id, Some(path), Either::Left(e))
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

#[derive(Debug)]
pub(super) struct WatchBuiltIn {
    id: BindId,
    top_id: ExprId,
    interest: BitFlags<Interest>,
    rec: bool,
    path: Option<ArcStr>,
}

impl<R: Rt, E: UserEvent> BuiltIn<R, E> for WatchBuiltIn {
    const NAME: &str = "is_err";
    deftype!(
        "core",
        "fn(?#interest:Array<Interest>, ?#recursive:bool, string) -> Result<string, `WatchError(string)>"
    );

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|ctx, _, _, _, top_id| {
            let id = BindId::new();
            ctx.rt.ref_var(id, top_id);
            Ok(Box::new(WatchBuiltIn {
                id,
                top_id,
                interest: BitFlags::empty(),
                rec: false,
                path: None,
            }))
        })
    }
}

impl<R: Rt, E: UserEvent> Apply<R, E> for WatchBuiltIn {
    fn update(
        &mut self,
        ctx: &mut ExecCtx<R, E>,
        from: &mut [Node<R, E>],
        event: &mut Event<E>,
    ) -> Option<Value> {
        let mut up = false;
        if let Some(Ok(int)) =
            from[0].update(ctx, event).map(|v| v.cast_to::<BitFlags<Interest>>())
        {
            up |= self.interest != int;
            self.interest = int;
        }
        if let Some(Ok(rec)) = from[1].update(ctx, event).map(|v| v.cast_to::<bool>()) {
            up |= self.rec != rec;
            self.rec = rec;
        }
        let mut path_up = false;
        if let Some(Ok(path)) = from[2].update(ctx, event).map(|v| v.cast_to::<ArcStr>())
        {
            let path = Some(path);
            path_up = path != self.path;
            self.path = path;
        }
        if path_up && let Some(path) = &self.path {
            ctx.rt.set_var(self.id, path.clone().into());
        }
        if (up || path_up)
            && let Some(path) = &self.path
        {
            let wctx = get_watcher(&mut ctx.rt, &mut ctx.libstate);
            if let Err(e) = wctx.send(WatchCmd::Watch(Watch {
                path: path.clone(),
                canonical_path: PathBuf::new(),
                id: self.id,
                interest: self.interest,
                recursive: self.rec,
            })) {
                ctx.rt.set_var(self.id, errf!("WatchError", "{e:?}"));
            }
        }
        if let Some(mut w) = event.custom.remove(&self.id)
            && let Some(w) = WatchEvent::downcast_mut(&mut w)
            && let Some(path) = &self.path
        {
            match &w.event {
                Ok(_) => ctx.rt.set_var(self.id, path.clone().into()),
                Err(e) => ctx.rt.set_var(self.id, errf!("WatchError", "{e:?}")),
            }
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
