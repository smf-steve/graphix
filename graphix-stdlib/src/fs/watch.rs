use crate::deftype;
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
    collections::hash_set,
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    result::Result,
    sync::{Arc, LazyLock},
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
    canonical_path: PathBuf,
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

/// like fs::cananocialize, but will never fail. It will canonicalize
/// as much of the path as it's possible to canonicalize and leave the
/// rest untouched.
async fn best_effort_canonicalize(path: &Path) -> PathBuf {
    let mut skipped: LPooled<Vec<&OsStr>> = LPooled::take();
    let mut root: &Path = &path;
    macro_rules! finish {
        ($p:expr) => {{
            for part in skipped.drain(..) {
                $p.push(part)
            }
            break $p;
        }};
    }
    loop {
        match fs::canonicalize(root).await {
            Ok(mut p) => finish!(p),
            Err(_) => match (root.parent(), root.file_name()) {
                (None, None) => break PathBuf::from(path),
                (None, Some(_)) => {
                    let mut p = PathBuf::from(root);
                    finish!(p)
                }
                (Some(parent), None) => root = parent,
                (Some(parent), Some(file)) => {
                    skipped.push(file);
                    root = parent;
                }
            },
        }
    }
}

fn utf8_path(path: &PathBuf) -> ArcStr {
    let path = path.as_os_str().as_bytes();
    match str::from_utf8(path) {
        Ok(s) => ArcStr::from(s),
        Err(_) => ArcStr::from(CompactString::from_utf8_lossy(path).as_str()),
    }
}

enum AddAction {
    AddWatch(PathBuf),
    JustNotify(PathBuf),
}

#[derive(Default)]
struct Watched {
    by_id: FxHashMap<BindId, Watch>,
    by_root: FxHashMap<PathBuf, FxHashSet<BindId>>,
}

impl Watched {
    /// add a watch and return the canonicalized path if adding a watch is necessary
    async fn add_watch(&mut self, mut w: Watch) -> AddAction {
        let id = w.id;
        let path = match self.by_id.get_mut(&id) {
            Some(ow) if ow.path == w.path => {
                let Watch { path: _, canonical_path, id: _, interest } = ow;
                let canonical_path = canonical_path.clone();
                *interest = w.interest;
                return AddAction::JustNotify(canonical_path);
            }
            Some(_) | None => best_effort_canonicalize(&Path::new(&*w.path)).await,
        };
        w.canonical_path = path.clone();
        self.by_id.insert(id, w);
        self.by_root.entry(path.clone()).or_default().insert(id);
        AddAction::AddWatch(path)
    }

    /// remove a watch, and return an optional action to be performed on the
    /// watcher
    fn remove_watch(&mut self, id: &BindId) -> Option<PathBuf> {
        self.by_id.remove(id).and_then(|w| {
            match self.by_root.get_mut(&w.canonical_path) {
                None => Some(w.canonical_path),
                Some(ids) => {
                    ids.remove(id);
                    ids.is_empty().then(|| {
                        self.by_root.remove(&w.canonical_path);
                        w.canonical_path
                    })
                }
            }
        })
    }

    fn relevant_to<'a>(&'a self, path: &'a PathBuf) -> impl Iterator<Item = &'a Watch> {
        struct I<'a> {
            root_ids: Option<hash_set::Iter<'a, BindId>>,
            path_ids: Option<hash_set::Iter<'a, BindId>>,
            t: &'a Watched,
        }
        impl<'a> Iterator for I<'a> {
            type Item = &'a Watch;

            fn next(&mut self) -> Option<Self::Item> {
                macro_rules! next {
                    ($set:expr) => {
                        if let Some(set) = $set
                            && let Some(id) = set.next()
                        {
                            match self.t.by_id.get(id) {
                                Some(w) => return Some(w),
                                None => continue,
                            }
                        }
                    };
                }
                loop {
                    next!(&mut self.path_ids);
                    next!(&mut self.root_ids);
                    return None;
                }
            }
        }
        I {
            path_ids: self.by_root.get(path).map(|h| h.iter()),
            root_ids: path.parent().and_then(|p| self.by_root.get(p).map(|h| h.iter())),
            t: self,
        }
    }

    fn process_event(
        &mut self,
        batch: &mut GPooled<Vec<(BindId, Box<dyn CustomBuiltinType>)>>,
        ev: Result<notify::Event, notify::Error>,
    ) {
        let mut by_id: LPooled<FxHashMap<BindId, WatchEvent>> = LPooled::take();
        match ev {
            Ok(ev) => {
                let event = WatchEventKind::Event((&ev.kind).into());
                for path in ev.paths.iter() {
                    let utf8_path = utf8_path(path);
                    for w in self.relevant_to(path) {
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
                            event: WatchEventKind::Error(err.clone()),
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
    eprintln!("starting file watcher loop");
    let mut watched = Watched::default();
    let mut recv_buf = vec![];
    let mut batch = CBATCH_POOL.take();
    macro_rules! or_push {
        ($path:expr, $id:expr, $r:expr) => {
            if let Err(e) = $r {
                let path = utf8_path(&$path);
                push_error(&mut batch, $id, Some(path), Either::Left(e))
            }
        };
    }
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
                    match watched.add_watch(w).await {
                        AddAction::JustNotify(path) => push_established(&mut batch, id, &path),
                        AddAction::AddWatch(path) => {
                            match watcher.watch(&path, RecursiveMode::NonRecursive) {
                                Ok(()) => push_established(&mut batch, id, &path),
                                Err(e) => push_error(
                                    &mut batch,
                                    id,
                                    Some(utf8_path(&path)),
                                    Either::Left(e)
                                )
                            }
                        }
                    }
                },
                WatchCmd::Stop(id) => match watched.remove_watch(&id) {
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

#[derive(Debug)]
pub(super) struct WatchBuiltIn {
    id: BindId,
    top_id: ExprId,
    interest: Option<BitFlags<Interest>>,
    path: Option<ArcStr>,
}

impl<R: Rt, E: UserEvent> BuiltIn<R, E> for WatchBuiltIn {
    const NAME: &str = "fs_watch";
    deftype!(
        "fs",
        "fn(?#interest:Array<Interest>, string) -> Result<string, `WatchError(string)>"
    );

    fn init(_: &mut ExecCtx<R, E>) -> BuiltInInitFn<R, E> {
        Arc::new(|ctx, _, _, _, top_id| {
            let id = BindId::new();
            ctx.rt.ref_var(id, top_id);
            Ok(Box::new(WatchBuiltIn { id, top_id, interest: None, path: None }))
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
        if let Some(Ok(mut int)) =
            from[0].update(ctx, event).map(|v| v.cast_to::<LPooled<Vec<Interest>>>())
        {
            let int = int.drain(..).fold(BitFlags::empty(), |mut acc, fl| {
                acc.insert(fl);
                acc
            });
            up |= self.interest != Some(int);
            self.interest = Some(int);
        }
        if let Some(Ok(path)) = from[1].update(ctx, event).map(|v| v.cast_to::<ArcStr>())
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
                canonical_path: PathBuf::new(),
                id: self.id,
                interest,
            })) {
                ctx.rt.set_var(self.id, errf!("WatchError", "{e:?}"));
            }
        }
        if let Some(mut w) = event.custom.remove(&self.id)
            && let Some(w) = (&mut *w as &mut dyn Any).downcast_mut::<WatchEvent>()
            && let Some(path) = &self.path
        {
            match &w.event {
                WatchEventKind::Established | WatchEventKind::Event(_) => {
                    ctx.rt.set_var(self.id, path.clone().into())
                }
                WatchEventKind::Error(e) => {
                    ctx.rt.set_var(self.id, errf!("WatchError", "{e:?}"))
                }
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
