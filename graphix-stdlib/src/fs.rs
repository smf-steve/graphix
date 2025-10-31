use arcstr::ArcStr;
use enumflags2::{bitflags, BitFlags};
use futures::{channel::mpsc, SinkExt, StreamExt};
use fxhash::{FxHashMap, FxHashSet};
use graphix_compiler::{err, BindId, CBATCH_POOL};
use netidx_value::Value;
use notify::{
    event::{
        AccessKind, CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind,
        RenameMode,
    },
    EventKind, RecommendedWatcher,
};
use poolshark::global::GPooled;
use std::{
    collections::hash_set,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::{fs, select, sync::mpsc as tmpsc};

#[derive(Debug, Clone, Copy)]
#[bitflags]
#[repr(u64)]
enum Interest {
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
            EventKind::Any => true,
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

impl notify::EventHandler for NotifyChan {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let _ = self.0.blocking_send(event);
    }
}

/// like fs::cananocialize, but will never fail. It will canonicalize
/// as much of the path as it's possible to canonicalize and leave the
/// rest untouched.
fn best_effort_canonicalize(path: PathBuf) -> Pin<Box<dyn Future<Output = PathBuf>>> {
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

    /// return an iterator over all watches triggered by a particular event
    fn triggered_by<'a>(
        &'a self,
        event: &'a notify::Event,
    ) -> impl Iterator<Item = &'a Watch> {
        event
            .paths
            .iter()
            .flat_map(|p| self.relevant_to(p))
            .filter(|w| w.interested(&event.kind))
    }

    /// add a watch and return the canonicalized path
    async fn add_watch(&mut self, mut w: Watch) -> PathBuf {
        let id = w.id;
        let path = best_effort_canonicalize(PathBuf::from(&*w.path)).await;
        w.canonical_path = path.clone();
        self.by_id.insert(id, w);
        self.by_root.entry(path.clone()).or_default().insert(id);
        path
    }

    /// remove a watch, and return it's root path if it is the last
    /// watch with that root
    async fn remove_watch(&mut self, id: &BindId) -> Option<PathBuf> {
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
}

async fn file_watcher_loop(
    mut watcher: RecommendedWatcher,
    mut rx_notify: tmpsc::Receiver<notify::Result<notify::Event>>,
    mut rx: mpsc::UnboundedReceiver<WatchCmd>,
    mut tx: mpsc::Sender<GPooled<Vec<(BindId, Value)>>>,
) {
    let mut watched = Watched::default();
    let mut recv_buf = vec![];
    loop {
        select! {
            n = rx_notify.recv_many(&mut recv_buf, 10000) => {
                if n == 0 {
                    break
                }
                let mut batch = CBATCH_POOL.take();
                for ev in recv_buf.drain(..) {
                    match ev {
                        Ok(ev) => (),
                        Err(ev) => {

                        }
                    }
                }
            },
            cmd = rx.next() => (),
        }
    }
    let mut batch = CBATCH_POOL.take();
    while let Ok(Some(cmd)) = rx.try_next() {
        match cmd {
            WatchCmd::Stop(_) => (),
            WatchCmd::Watch(w) => {
                batch.push((w.id, err!("WatchError", "watcher thread has stopped")));
            }
        }
    }
    if !batch.is_empty() {
        let _ = tx.send(batch).await;
    }
}

struct WatchCtx(mpsc::UnboundedSender<WatchCmd>);

struct NotifyChan(tmpsc::Sender<notify::Result<notify::Event>>);
