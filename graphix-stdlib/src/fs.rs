use arcstr::ArcStr;
use enumflags2::{bitflags, BitFlags};
use futures::{channel::mpsc, SinkExt, StreamExt};
use fxhash::FxHashMap;
use graphix_compiler::{err, BindId, CBATCH_POOL};
use netidx_value::Value;
use notify::RecommendedWatcher;
use poolshark::global::GPooled;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    slice,
};
use tokio::{select, sync::mpsc as tmpsc};

#[derive(Debug, Clone, Copy)]
#[bitflags]
#[repr(u8)]
enum Interest {
    Create,
    Modify,
    Delete,
    Rename,
}

#[derive(Debug, Clone)]
struct Watch {
    path: ArcStr,
    id: BindId,
    interest: BitFlags<Interest>,
    recursive: bool,
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

#[derive(Default)]
struct Watched {
    by_id: FxHashMap<BindId, Watch>,
    by_root: FxHashMap<PathBuf, Vec<BindId>>,
}

impl Watched {
    fn relevant_watches<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = &'a Watch> {
        struct I<'a> {
            t: &'a Watched,
            path: Option<&'a Path>,
            curr: slice::Iter<'a, BindId>,
        }
        impl<'a> Iterator for I<'a> {
            type Item = &'a Watch;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    match self.path {
                        None => break None,
                        Some(path) => match self.t.by_root.get(path) {
                            None => self.path = path.parent(),
                            Some(id) => {
                                self.path = path.parent();
                                if let Some(w) = self.t.by_id.get(id) {
                                    break Some(w);
                                }
                            }
                        },
                    }
                }
            }
        }
        I { t: self, path: Some(path) }
    }
}

async fn file_watcher_loop(
    mut watcher: RecommendedWatcher,
    mut rx_notify: tmpsc::Receiver<notify::Result<notify::Event>>,
    mut rx: mpsc::UnboundedReceiver<WatchCmd>,
    mut tx: mpsc::Sender<GPooled<Vec<(BindId, Value)>>>,
) {
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
