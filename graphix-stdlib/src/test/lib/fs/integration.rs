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

    // First write
    let code =
        format!(r#"fs::write_all(#path: "{}", "Test content")"#, test_file.display());
    let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
    let write_eid = compiled.exprs[0].id;

    // Wait for write to complete
    let timeout = tokio::time::sleep(Duration::from_secs(2));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => panic!("timeout waiting for write"),
            Some(mut batch) = rx.recv() => {
                for event in batch.drain(..) {
                    if let GXEvent::Updated(id, v) = event {
                        if id == write_eid && matches!(v, Value::Null) {
                            // Write succeeded, now read
                            let code = format!(r#"fs::read_all("{}")"#, test_file.display());
                            let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
                            let read_eid = compiled.exprs[0].id;

                            // Wait for read result
                            let timeout = tokio::time::sleep(Duration::from_secs(2));
                            tokio::pin!(timeout);

                            loop {
                                tokio::select! {
                                    _ = &mut timeout => panic!("timeout waiting for read"),
                                    Some(mut batch) = rx.recv() => {
                                        for event in batch.drain(..) {
                                            if let GXEvent::Updated(id, v) = event {
                                                if id == read_eid {
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

                                // Now write to the file using fs::write_all
                                let code = format!(
                                    r#"fs::write_all(#path: "{}", "modified by write_all")"#,
                                    test_file.display()
                                );
                                let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
                                write_eid = Some(compiled.exprs[0].id);
                            } else {
                                eprintln!("Got modify event: {v}");
                                got_modify_event = true;
                            }
                        }

                        // Also check for write completion
                        if let Some(wid) = write_eid {
                            if id == wid && matches!(v, Value::Null) {
                                eprintln!("Write completed successfully");
                            }
                        }
                    }
                }
            }
        }
    }

    assert!(watch_established, "Watch was not established");
    assert!(got_modify_event, "Did not receive modify event after write");

    // Verify file was actually modified
    let content = fs::read_to_string(&test_file).await?;
    assert_eq!(content, "modified by write_all");

    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn test_write_bin_then_read_bin() -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
    let ctx = init(tx).await?;
    let temp_dir = tempfile::tempdir()?;
    let test_file = temp_dir.path().join("binary_cycle.bin");

    // Write binary data (bytes:SGVsbG8= is "Hello" in base64)
    let code =
        format!(r#"fs::write_all_bin(#path: "{}", bytes:SGVsbG8=)"#, test_file.display());
    let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
    let write_eid = compiled.exprs[0].id;

    let timeout = tokio::time::sleep(Duration::from_secs(2));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            _ = &mut timeout => panic!("timeout waiting for write"),
            Some(mut batch) = rx.recv() => {
                for event in batch.drain(..) {
                    if let GXEvent::Updated(id, v) = event {
                        if id == write_eid && matches!(v, Value::Null) {
                            // Write succeeded, now read
                            let code = format!(r#"fs::read_all_bin("{}")"#, test_file.display());
                            let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
                            let read_eid = compiled.exprs[0].id;

                            let timeout = tokio::time::sleep(Duration::from_secs(2));
                            tokio::pin!(timeout);

                            loop {
                                tokio::select! {
                                    _ = &mut timeout => panic!("timeout waiting for read"),
                                    Some(mut batch) = rx.recv() => {
                                        for event in batch.drain(..) {
                                            if let GXEvent::Updated(id, v) = event {
                                                if id == read_eid {
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
                    }
                }
            }
        }
    }
}
