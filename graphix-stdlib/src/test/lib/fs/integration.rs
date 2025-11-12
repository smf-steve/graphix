use anyhow::Result;
use arcstr::ArcStr;
use netidx::subscriber::Value;
use tokio::fs;
use tokio::time::Duration;

use crate::test::init;
use graphix_rt::GXEvent;
use poolshark::global::GPooled;
use tokio::sync::mpsc;

#[tokio::test(flavor = "current_thread")]
async fn test_write_then_read() -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
    let ctx = init(tx).await?;
    let temp_dir = tempfile::tempdir()?;
    let test_file = temp_dir.path().join("write_read_test.txt");

    // Use Graphix to write and then read the file in one expression
    // Use the sample operator (~) to sequence the read after the write completes
    let code = format!(
        r#"{{
  let path = "{}";
  let write_result = fs::write_all(#path: path, "Test content");
  fs::read_all(write_result ~ path)
}}"#,
        test_file.display()
    );

    let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
    let eid = compiled.exprs[0].id;

    let timeout = tokio::time::sleep(Duration::from_secs(2));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => panic!("timeout waiting for result"),
            Some(mut batch) = rx.recv() => {
                for event in batch.drain(..) {
                    if let GXEvent::Updated(id, v) = event {
                        if id == eid {
                            if let Value::String(s) = v {
                                assert_eq!(&*s, "Test content");
                                return Ok(());
                            }
                            panic!("expected String, got: {v:?}");
                        }
                    }
                }
            }
        }
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_write_then_watch_modify() -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
    let ctx = init(tx).await?;
    let temp_dir = tempfile::tempdir()?;
    let watch_path = temp_dir.path().to_str().unwrap();
    let test_file = temp_dir.path().join("watch_write_test.txt");

    // Create initial file
    fs::write(&test_file, "initial").await?;

    // Start watching for modifications
    let code =
        format!(r#"fs::watch(#interest: [`Established, `Modify], "{}")"#, watch_path);
    let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
    let watch_eid = compiled.exprs[0].id;

    let timeout = tokio::time::sleep(Duration::from_secs(3));
    tokio::pin!(timeout);
    let mut watch_established = false;
    let mut got_modify_event = false;
    let mut write_eid = None;

    loop {
        tokio::select! {
            _ = &mut timeout => break,
            Some(mut batch) = rx.recv() => {
                for event in batch.drain(..) {
                    if let GXEvent::Updated(id, v) = event {
                        if id == watch_eid && matches!(v, Value::String(_)) {
                            if !watch_established {
                                watch_established = true;
                                eprintln!("Watch established, performing write");

                                // Now write to the file and read it back using Graphix
                                let code = format!(
                                    r#"{{
  let path = "{}";
  let write_result = fs::write_all(#path: path, "modified by write_all");
  fs::read_all(path)
}}"#,
                                    test_file.display()
                                );
                                let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
                                write_eid = Some(compiled.exprs[0].id);
                            } else {
                                eprintln!("Got modify event: {v}");
                                got_modify_event = true;
                            }
                        }

                        // Check for write+read completion
                        if let Some(wid) = write_eid {
                            if id == wid {
                                if let Value::String(s) = &v {
                                    assert_eq!(&**s, "modified by write_all");
                                    eprintln!("Write and read completed successfully");
                                } else {
                                    panic!("Expected string from read, got: {v:?}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(watch_established, "Watch was not established");
    assert!(got_modify_event, "Did not receive modify event after write");

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_write_bin_then_read_bin() -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
    let ctx = init(tx).await?;
    let temp_dir = tempfile::tempdir()?;
    let test_file = temp_dir.path().join("binary_cycle.bin");

    // Use Graphix to write binary and then read it back in one expression
    // Use the sample operator (~) to sequence the read after the write completes
    let code = format!(
        r#"{{
  let path = "{}";
  let write_result = fs::write_all_bin(#path: path, bytes:SGVsbG8=);
  fs::read_all_bin(write_result ~ path)
}}"#,
        test_file.display()
    );

    let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
    let eid = compiled.exprs[0].id;

    let timeout = tokio::time::sleep(Duration::from_secs(2));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => panic!("timeout waiting for result"),
            Some(mut batch) = rx.recv() => {
                for event in batch.drain(..) {
                    if let GXEvent::Updated(id, v) = event {
                        if id == eid {
                            if let Value::Bytes(b) = v {
                                assert_eq!(b.as_ref(), b"Hello");
                                return Ok(());
                            }
                            panic!("expected Bytes, got: {v:?}");
                        }
                    }
                }
            }
        }
    }
}
