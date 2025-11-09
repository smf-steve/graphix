use anyhow::Result;
use arcstr::ArcStr;
use netidx::subscriber::Value;
use tokio::fs;
use tokio::time::Duration;

#[cfg(test)]
mod watch_tests {
    use super::*;
    use crate::test::init;
    use graphix_rt::GXEvent;
    use poolshark::global::GPooled;
    use tokio::sync::mpsc;

    /// Macro to create fs::watch tests with common setup/teardown logic
    macro_rules! watch_test {
        (
            name: $test_name:ident,
            interest: $interest:expr,
            setup: |$temp_dir:ident| $setup:block,
            action: |$action_dir:ident| $action:block,
            expect: $expect:expr
        ) => {
            #[tokio::test(flavor = "current_thread")]
            async fn $test_name() -> Result<()> {
                let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
                let ctx = init(tx).await?;
                let $temp_dir = tempfile::tempdir()?;
                let watch_path = $temp_dir.path().to_str().unwrap();

                // Run setup block
                $setup

                // Start watching
                let code = format!(
                    r#"fs::watch(#interest: {}, "{}")"#,
                    $interest, watch_path
                );

                let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
                let eid = compiled.exprs[0].id;

                // Wait for events
                // First event will be the initial path return when watch starts
                // Subsequent events are actual filesystem events
                let mut event_count = 0;
                let timeout = tokio::time::sleep(Duration::from_secs(2));
                tokio::pin!(timeout);

                loop {
                    tokio::select! {
                        _ = &mut timeout => break,
                        Some(mut batch) = rx.recv() => {
                            for event in batch.drain(..) {
                                match event {
                                    GXEvent::Env(_) => (),
                                    GXEvent::Updated(id, v) => {
                                        eprintln!("got event {}: {v}", id.inner());
                                        if id == eid {
                                            if matches!(v, Value::String(_)) {
                                                event_count += 1;
                                                eprintln!("Watch event #{}: {}", event_count, v);
                                                if event_count == 1 {
                                                    eprintln!("watch established, performing action");
                                                    let $action_dir = &$temp_dir;
                                                    $action
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // event_count > 1 means we got filesystem events after the initial path return
                let got_event = event_count > 1;

                assert_eq!(got_event, $expect,
                    "Expected event: {}, Got event: {}", $expect, got_event);
                Ok(())
            }
        };
    }

    // Test file creation detection
    watch_test! {
        name: test_watch_create_file,
        interest: "[`Create]",
        setup: |_temp_dir| {},
        action: |temp_dir| {
            let test_file = temp_dir.path().join("test_file.txt");
            fs::write(&test_file, b"hello").await?;
        },
        expect: true
    }

    // Test file modification detection
    watch_test! {
        name: test_watch_modify_file,
        interest: "[`Modify]",
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("test_file.txt");
            fs::write(&test_file, b"initial").await?;
        },
        action: |temp_dir| {
            let test_file = temp_dir.path().join("test_file.txt");
            fs::write(&test_file, b"modified content").await?;
        },
        expect: true
    }

    // Test file deletion detection
    watch_test! {
        name: test_watch_delete_file,
        interest: "[`Delete]",
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("test_file.txt");
            fs::write(&test_file, b"to be deleted").await?;
        },
        action: |temp_dir| {
            let test_file = temp_dir.path().join("test_file.txt");
            fs::remove_file(&test_file).await?;
        },
        expect: true
    }

    // Test interest filtering (should NOT detect events not matching interest)
    watch_test! {
        name: test_watch_interest_filtering,
        interest: "[`Create]",
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("test_file.txt");
            fs::write(&test_file, b"initial").await?;
        },
        action: |temp_dir| {
            let test_file = temp_dir.path().join("test_file.txt");
            fs::write(&test_file, b"modified").await?;
        },
        expect: false
    }

    // Test watching a non-existent file that gets created
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_nonexistent_file_created() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;
        let test_file = temp_dir.path().join("nonexistent.txt");

        // Ensure file doesn't exist
        let _ = fs::remove_file(&test_file).await;

        // Watch the non-existent file
        let code = format!(
            r#"fs::watch(#interest: [`Create, `Established], "{}")"#,
            test_file.display()
        );

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(5));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut file_created = false;
        let mut got_create = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid {
                                eprintln!("Event: {v}");
                                if matches!(v, Value::String(_)) {
                                    event_count += 1;
                                    if event_count == 1 && !file_created {
                                        // First event - watch is established as pending
                                        // Now create the file
                                        eprintln!("Creating file");
                                        fs::write(&test_file, b"hello").await?;
                                        file_created = true;
                                    } else if event_count >= 2 {
                                        // Should get the create event
                                        got_create = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_create, "Did not receive create event for non-existent file");
        Ok(())
    }

    // Test watching existing file, deleting it, then recreating it
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_delete_then_recreate() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;
        let test_file = temp_dir.path().join("delete_recreate.txt");

        // Create initial file
        fs::write(&test_file, b"initial").await?;

        // Watch for all events
        let code = format!(
            r#"fs::watch(#interest: [`Create, `Delete, `Modify], "{}")"#,
            test_file.display()
        );

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(6));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_delete = false;
        let mut got_create = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established, delete the file
                                    eprintln!("Deleting file");
                                    fs::remove_file(&test_file).await?;
                                } else if event_count == 2 {
                                    // Should be delete event
                                    got_delete = true;
                                    eprintln!("Recreating file");
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    fs::write(&test_file, b"recreated").await?;
                                } else if event_count == 3 {
                                    // Should be create event
                                    got_create = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_delete, "Did not receive delete event");
        assert!(got_create, "Did not receive create event after recreation");
        Ok(())
    }

    // Test renaming parent directory
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_parent_rename() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;
        let parent_dir = temp_dir.path().join("parent");
        fs::create_dir(&parent_dir).await?;
        let test_file = parent_dir.join("file.txt");
        fs::write(&test_file, b"content").await?;

        // Watch the file
        let code = format!(
            r#"fs::watch(#interest: [`Delete, `Create], "{}")"#,
            test_file.display()
        );

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(4));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_delete = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established, rename parent
                                    let new_parent = temp_dir.path().join("parent_renamed");
                                    eprintln!("Renaming parent directory");
                                    fs::rename(&parent_dir, &new_parent).await?;
                                } else {
                                    // Should get delete event (via polling)
                                    got_delete = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_delete, "Did not receive delete event after parent rename");
        Ok(())
    }

    // Test multi-level parent creation
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_multilevel_parent_creation() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;
        let deep_file = temp_dir.path().join("a/b/c/file.txt");

        // Ensure nothing exists
        let _ = fs::remove_dir_all(temp_dir.path().join("a")).await;

        // Watch the deep file
        let code =
            format!(r#"fs::watch(#interest: [`Create], "{}")"#, deep_file.display());

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(5));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_create = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established as pending
                                    // Create directories one by one
                                    eprintln!("Creating /a");
                                    fs::create_dir(temp_dir.path().join("a")).await?;
                                    tokio::time::sleep(Duration::from_millis(100)).await;

                                    eprintln!("Creating /a/b");
                                    fs::create_dir(temp_dir.path().join("a/b")).await?;
                                    tokio::time::sleep(Duration::from_millis(100)).await;

                                    eprintln!("Creating /a/b/c");
                                    fs::create_dir(temp_dir.path().join("a/b/c")).await?;
                                    tokio::time::sleep(Duration::from_millis(100)).await;

                                    eprintln!("Creating file");
                                    fs::write(&deep_file, b"deep content").await?;
                                } else {
                                    // Should get create event when file appears
                                    got_create = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_create, "Did not receive create event for deep file");
        Ok(())
    }

    // Test deep parent rename (rename two levels up)
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_deep_parent_rename() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;

        // Create deep structure /a/b/c/d/file.txt
        let a = temp_dir.path().join("a");
        let b = a.join("b");
        let c = b.join("c");
        let d = c.join("d");
        fs::create_dir_all(&d).await?;
        let test_file = d.join("file.txt");
        fs::write(&test_file, b"content").await?;

        // Watch the file
        let code =
            format!(r#"fs::watch(#interest: [`Delete], "{}")"#, test_file.display());

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(4));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_delete = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established, rename /a/b to /a/b2
                                    let b2 = a.join("b2");
                                    eprintln!("Renaming /a/b to /a/b2 (two levels up)");
                                    fs::rename(&b, &b2).await?;
                                } else {
                                    // Should get delete event
                                    got_delete = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_delete, "Did not receive delete event after deep parent rename");
        Ok(())
    }

    // Test race with parent deletion
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_parent_tree_deletion() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;

        // Create structure /a/b/file.txt
        let a = temp_dir.path().join("a");
        let b = a.join("b");
        fs::create_dir_all(&b).await?;
        let test_file = b.join("file.txt");
        fs::write(&test_file, b"content").await?;

        // Watch the file
        let code =
            format!(r#"fs::watch(#interest: [`Delete], "{}")"#, test_file.display());

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(4));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_delete = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established, delete entire /a tree
                                    eprintln!("Deleting entire /a directory tree");
                                    fs::remove_dir_all(&a).await?;
                                } else {
                                    // Should get delete event
                                    got_delete = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_delete, "Did not receive delete event after parent tree deletion");
        Ok(())
    }

    // Test multiple watches on related paths
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_multiple_related_paths() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;

        let file1 = temp_dir.path().join("a/b/file.txt");
        let file2 = temp_dir.path().join("a/b/c/file.txt");

        // Ensure nothing exists
        let _ = fs::remove_dir_all(temp_dir.path().join("a")).await;

        // Watch both files
        let code1 = format!(r#"fs::watch(#interest: [`Create], "{}")"#, file1.display());
        let code2 = format!(r#"fs::watch(#interest: [`Create], "{}")"#, file2.display());

        let compiled1 = ctx.rt.compile(ArcStr::from(code1)).await?;
        let compiled2 = ctx.rt.compile(ArcStr::from(code2)).await?;
        let eid1 = compiled1.exprs[0].id;
        let eid2 = compiled2.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(5));
        tokio::pin!(timeout);
        let mut watch1_ready = false;
        let mut watch2_ready = false;
        let mut got_create_file1 = false;
        let mut got_create_file2 = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if matches!(v, Value::String(_)) {
                                eprintln!("Event for {}: {v}", id.inner());

                                if id == eid1 && !watch1_ready {
                                    watch1_ready = true;
                                    eprintln!("Watch 1 ready");
                                } else if id == eid2 && !watch2_ready {
                                    watch2_ready = true;
                                    eprintln!("Watch 2 ready");
                                } else if id == eid1 {
                                    got_create_file1 = true;
                                } else if id == eid2 {
                                    got_create_file2 = true;
                                }

                                if watch1_ready && watch2_ready && !got_create_file1 && !got_create_file2 {
                                    // Both watches ready, create only file2
                                    eprintln!("Creating deep file only (file2)");
                                    fs::create_dir_all(temp_dir.path().join("a/b/c")).await?;
                                    fs::write(&file2, b"deep").await?;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(!got_create_file1, "Should not get create event for file1");
        assert!(got_create_file2, "Should get create event for file2");
        Ok(())
    }

    // Test established -> pending transition
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_established_to_pending() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;

        // Create file in subdirectory
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).await?;
        let test_file = subdir.join("file.txt");
        fs::write(&test_file, b"content").await?;

        // Watch with Established interest
        let code = format!(
            r#"fs::watch(#interest: [`Delete, `Established], "{}")"#,
            test_file.display()
        );

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(4));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_established = false;
        let mut got_delete = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Should get Established event
                                    got_established = true;
                                    eprintln!("Deleting parent directory");
                                    fs::remove_dir_all(&subdir).await?;
                                } else {
                                    // Should get Delete event
                                    got_delete = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_established, "Did not receive Established event");
        assert!(got_delete, "Did not receive Delete event after parent deletion");
        Ok(())
    }

    // Test file -> directory transition
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_file_to_directory() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path().join("transform");

        // Create as regular file
        fs::write(&path, b"file content").await?;

        // Watch it
        let code =
            format!(r#"fs::watch(#interest: [`Delete, `Create], "{}")"#, path.display());

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(4));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_delete = false;
        let mut got_create = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established, transform to directory
                                    eprintln!("Deleting file and creating directory");
                                    fs::remove_file(&path).await?;
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    fs::create_dir(&path).await?;
                                } else if event_count == 2 {
                                    got_delete = true;
                                } else if event_count == 3 {
                                    got_create = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_delete, "Did not receive delete event");
        assert!(got_create, "Did not receive create event for directory");
        Ok(())
    }

    // Test symlink with non-existent target
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_symlink_nonexistent_target() -> Result<()> {
        use std::os::unix::fs::symlink;

        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;
        let target = temp_dir.path().join("target.txt");
        let link = temp_dir.path().join("link.txt");

        // Create symlink to non-existent target
        symlink(&target, &link)?;

        // Watch the symlink
        let code =
            format!(r#"fs::watch(#interest: [`Create, `Modify], "{}")"#, link.display());

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(4));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_event = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established, create the target
                                    eprintln!("Creating symlink target");
                                    fs::write(&target, b"target content").await?;
                                } else {
                                    // Should get event when target appears
                                    got_event = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_event, "Did not receive event when symlink target was created");
        Ok(())
    }

    // Test deleting and recreating symlink itself
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_symlink_recreate() -> Result<()> {
        use std::os::unix::fs::symlink;

        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;
        let target = temp_dir.path().join("target.txt");
        let link = temp_dir.path().join("link.txt");

        // Create target and symlink
        fs::write(&target, b"content").await?;
        symlink(&target, &link)?;

        // Watch the symlink
        let code =
            format!(r#"fs::watch(#interest: [`Delete, `Create], "{}")"#, link.display());

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(4));
        tokio::pin!(timeout);
        let mut event_count = 0;
        let mut got_delete = false;
        let mut got_create = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established, delete and recreate symlink
                                    eprintln!("Deleting symlink");
                                    fs::remove_file(&link).await?;
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    eprintln!("Recreating symlink");
                                    symlink(&target, &link)?;
                                } else if event_count == 2 {
                                    got_delete = true;
                                } else if event_count == 3 {
                                    got_create = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(got_delete, "Did not receive delete event for symlink");
        assert!(got_create, "Did not receive create event for symlink");
        Ok(())
    }

    // Test rapid delete-recreate within poll interval
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_rapid_transitions() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;
        let temp_dir = tempfile::tempdir()?;
        let test_file = temp_dir.path().join("rapid.txt");

        // Create initial file
        fs::write(&test_file, b"v1").await?;

        // Watch it
        let code = format!(
            r#"fs::watch(#interest: [`Delete, `Create, `Modify], "{}")"#,
            test_file.display()
        );

        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(5));
        tokio::pin!(timeout);
        let mut event_count = 0;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid && matches!(v, Value::String(_)) {
                                event_count += 1;
                                eprintln!("Event #{event_count}: {v}");

                                if event_count == 1 {
                                    // Watch established, perform rapid transitions
                                    eprintln!("Performing rapid delete-recreate cycle");
                                    fs::remove_file(&test_file).await?;
                                    // Very short delay - faster than poll interval
                                    tokio::time::sleep(Duration::from_millis(10)).await;
                                    fs::write(&test_file, b"v2").await?;
                                }
                            }
                        }
                    }
                }
            }
        }

        // After rapid transition, file should exist with final content
        assert!(test_file.exists(), "File should exist after rapid transitions");
        let content = fs::read(&test_file).await?;
        assert_eq!(content, b"v2", "File should have final content");

        // We should get some events, but exact count depends on timing
        assert!(event_count >= 1, "Should get at least the initial event");
        eprintln!("Total events received: {event_count}");
        Ok(())
    }

    // Test SetGlobals - disable polling
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_set_globals_disable_polling() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;

        // Set poll_batch_size to 0 to disable polling
        let code = r#"fs::set_global_watch_parameters(#poll_batch_size: 0, #poll_interval: duration:1.s)"#;
        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(2));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                _ = &mut timeout => panic!("timeout waiting for set_watch_globals"),
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid {
                                eprintln!("set_watch_globals result: {v}");
                                // Should return Ok(null)
                                assert!(matches!(v, Value::Null), "Expected Null (success), got: {v:?}");
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    // Test SetGlobals - configure fast polling
    #[tokio::test(flavor = "current_thread")]
    async fn test_watch_set_globals_fast_polling() -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
        let ctx = init(tx).await?;

        // Set very fast polling (100ms interval, batch size 200)
        let code = r#"fs::set_global_watch_parameters(#poll_batch_size: 200u64, #poll_interval: 100u64)"#;
        let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
        let eid = compiled.exprs[0].id;

        let timeout = tokio::time::sleep(Duration::from_secs(2));
        tokio::pin!(timeout);
        let mut got_result = false;

        loop {
            tokio::select! {
                _ = &mut timeout => break,
                Some(mut batch) = rx.recv() => {
                    for event in batch.drain(..) {
                        if let GXEvent::Updated(id, v) = event {
                            if id == eid {
                                eprintln!("set_watch_globals result: {v}");
                                assert!(matches!(v, Value::Null), "Expected Null (success), got: {v:?}");
                                got_result = true;

                                // Now test that fast polling works by watching a non-existent file
                                let temp_dir = tempfile::tempdir()?;
                                let test_file = temp_dir.path().join("fast_poll.txt");

                                let watch_code = format!(
                                    r#"fs::watch(#interest: [`Create], "{}")"#,
                                    test_file.display()
                                );
                                let watch_compiled = ctx.rt.compile(ArcStr::from(watch_code)).await?;
                                let watch_eid = watch_compiled.exprs[0].id;

                                // Wait a bit for watch to establish
                                tokio::time::sleep(Duration::from_millis(200)).await;

                                // Create the file
                                eprintln!("Creating file with fast polling enabled");
                                fs::write(&test_file, b"fast").await?;

                                // With 100ms polling, should get event quickly
                                let timeout = tokio::time::sleep(Duration::from_millis(500));
                                tokio::pin!(timeout);

                                loop {
                                    tokio::select! {
                                        _ = &mut timeout => {
                                            panic!("timeout waiting for fast poll event");
                                        },
                                        Some(mut batch) = rx.recv() => {
                                            for event in batch.drain(..) {
                                                if let GXEvent::Updated(id, v) = event {
                                                    if id == watch_eid && matches!(v, Value::String(_)) {
                                                        eprintln!("Got watch event: {v}");
                                                        return Ok(());
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

        assert!(got_result, "Did not receive result from set_watch_globals");
        Ok(())
    }
}

#[cfg(test)]
mod read_tests {
    use super::*;
    use crate::{run, test::init};
    use anyhow::{bail, Result};
    use graphix_rt::GXEvent;
    use netidx::subscriber::Value;
    use poolshark::global::GPooled;
    use tokio::sync::mpsc;

    /// Macro to create fs::read_* tests with common setup/teardown logic
    macro_rules! read_test {
        (
            name: $test_name:ident,
            function: $func:expr,
            setup: |$temp_dir:ident| $setup:block,
            expect_ok: $expect_payload:expr
        ) => {
            #[tokio::test(flavor = "current_thread")]
            async fn $test_name() -> Result<()> {
                let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
                let ctx = init(tx).await?;
                let $temp_dir = tempfile::tempdir()?;

                // Run setup block which should return test_file
                let test_file = { $setup };

                let code = format!(r#"{}("{}")"#, $func, test_file.display());
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
                                        $expect_payload(v)?;
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
        (
            name: $test_name:ident,
            function: $func:expr,
            setup: |$temp_dir:ident| $setup:block,
            expect_error: true
        ) => {
            #[tokio::test(flavor = "current_thread")]
            async fn $test_name() -> Result<()> {
                let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
                let ctx = init(tx).await?;
                let $temp_dir = tempfile::tempdir()?;

                // Run setup block which should return test_file
                let test_file = { $setup };

                let code = format!(r#"{}("{}")"#, $func, test_file.display());
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
                                        if matches!(v, Value::Error(_)) {
                                            return Ok(());
                                        }
                                        panic!("expected Error value, got: {v:?}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
    }

    read_test! {
        name: test_read_all_basic,
        function: "fs::read_all",
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("test.txt");
            let content = "Hello, World!";
            fs::write(&test_file, content).await?;
            test_file
        },
        expect_ok: |v: Value| -> Result<()> {
            if let Value::String(s) = v {
                assert_eq!(&*s, "Hello, World!");
                Ok(())
            } else {
                panic!("expected String value, got: {v:?}")
            }
        }
    }

    read_test! {
        name: test_read_all_nonexistent,
        function: "fs::read_all",
        setup: |temp_dir| {
            temp_dir.path().join("nonexistent.txt")
        },
        expect_error: true
    }

    read_test! {
        name: test_read_all_utf8,
        function: "fs::read_all",
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("utf8.txt");
            let content = "Hello, 世界! 🦀";
            fs::write(&test_file, content).await?;
            test_file
        },
        expect_ok: |v: Value| -> Result<()> {
            if let Value::String(s) = v {
                assert_eq!(&*s, "Hello, 世界! 🦀");
                Ok(())
            } else {
                panic!("expected String value, got: {v:?}")
            }
        }
    }

    read_test! {
        name: test_read_all_bin_basic,
        function: "fs::read_all_bin",
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("test.bin");
            let content = b"Binary data \x00\x01\x02\xff";
            fs::write(&test_file, content).await?;
            test_file
        },
        expect_ok: |v: Value| -> Result<()> {
            if let Value::Bytes(b) = v {
                assert_eq!(b.as_ref(), b"Binary data \x00\x01\x02\xff");
                Ok(())
            } else {
                panic!("expected Bytes value, got: {v:?}")
            }
        }
    }

    read_test! {
        name: test_read_all_bin_nonexistent,
        function: "fs::read_all_bin",
        setup: |temp_dir| {
            temp_dir.path().join("nonexistent.bin")
        },
        expect_error: true
    }

    // Simple test using run! macro to verify basic functionality
    run!(
        test_read_all_simple,
        r#"fs::read_all("/tmp/graphix_test.txt")"#,
        |v: Result<&Value>| {
            match v {
                Ok(val) => {
                    // Result is a union type - either String (success) or Error (failure)
                    matches!(val, Value::String(_) | Value::Error(_))
                }
                Err(_) => false,
            }
        }
    );
}

#[cfg(test)]
mod write_tests {
    use super::*;
    use crate::test::init;
    use anyhow::Result;
    use graphix_rt::GXEvent;
    use netidx::subscriber::Value;
    use poolshark::global::GPooled;
    use tokio::sync::mpsc;

    /// Macro to create fs::write_* tests with common setup/teardown logic
    macro_rules! write_test {
        (
            name: $test_name:ident,
            function: $func:expr,
            content: $content:expr,
            setup: |$temp_dir:ident| $setup:block,
            verify: |$verify_dir:ident| $verify:block
        ) => {
            #[tokio::test(flavor = "current_thread")]
            async fn $test_name() -> Result<()> {
                let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
                let ctx = init(tx).await?;
                let $temp_dir = tempfile::tempdir()?;

                // Run setup block which should return test_file
                let test_file = { $setup };

                let code = format!(
                    r#"{}(#path: "{}", {})"#,
                    $func,
                    test_file.display(),
                    $content
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
                                        // Check write succeeded (returns Ok(null))
                                        if !matches!(v, Value::Null) {
                                            panic!("expected Null (success), got: {v:?}");
                                        }
                                        // Verify file contents
                                        let $verify_dir = &$temp_dir;
                                        $verify
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
        (
            name: $test_name:ident,
            function: $func:expr,
            content: $content:expr,
            setup: |$temp_dir:ident| $setup:block,
            expect_error: true
        ) => {
            #[tokio::test(flavor = "current_thread")]
            async fn $test_name() -> Result<()> {
                let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
                let ctx = init(tx).await?;
                let $temp_dir = tempfile::tempdir()?;

                // Run setup block which should return test_file
                let test_file = { $setup };

                let code = format!(
                    r#"{}(#path: "{}", {})"#,
                    $func,
                    test_file.display(),
                    $content
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
                                        if matches!(v, Value::Error(_)) {
                                            return Ok(());
                                        }
                                        panic!("expected Error value, got: {v:?}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
    }

    write_test! {
        name: test_write_all_basic,
        function: "fs::write_all",
        content: r#""Hello, World!""#,
        setup: |temp_dir| {
            temp_dir.path().join("test.txt")
        },
        verify: |temp_dir| {
            let test_file = temp_dir.path().join("test.txt");
            let content = fs::read_to_string(&test_file).await?;
            assert_eq!(content, "Hello, World!");
        }
    }

    write_test! {
        name: test_write_all_create_new_file,
        function: "fs::write_all",
        content: r#""New file content""#,
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("new_file.txt");
            // Ensure file doesn't exist
            let _ = fs::remove_file(&test_file).await;
            test_file
        },
        verify: |temp_dir| {
            let test_file = temp_dir.path().join("new_file.txt");
            assert!(test_file.exists());
            let content = fs::read_to_string(&test_file).await?;
            assert_eq!(content, "New file content");
        }
    }

    write_test! {
        name: test_write_all_overwrite_existing,
        function: "fs::write_all",
        content: r#""Overwritten content""#,
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("existing.txt");
            fs::write(&test_file, "Original content").await?;
            test_file
        },
        verify: |temp_dir| {
            let test_file = temp_dir.path().join("existing.txt");
            let content = fs::read_to_string(&test_file).await?;
            assert_eq!(content, "Overwritten content");
        }
    }

    write_test! {
        name: test_write_all_utf8,
        function: "fs::write_all",
        content: r#""Hello, 世界! 🦀""#,
        setup: |temp_dir| {
            temp_dir.path().join("utf8.txt")
        },
        verify: |temp_dir| {
            let test_file = temp_dir.path().join("utf8.txt");
            let content = fs::read_to_string(&test_file).await?;
            assert_eq!(content, "Hello, 世界! 🦀");
        }
    }

    write_test! {
        name: test_write_all_empty_string,
        function: "fs::write_all",
        content: r#""""#,
        setup: |temp_dir| {
            temp_dir.path().join("empty.txt")
        },
        verify: |temp_dir| {
            let test_file = temp_dir.path().join("empty.txt");
            let content = fs::read_to_string(&test_file).await?;
            assert_eq!(content, "");
        }
    }

    write_test! {
        name: test_write_all_bin_basic,
        function: "fs::write_all_bin",
        content: r#"bytes:SGVsbG8="#,
        setup: |temp_dir| {
            temp_dir.path().join("test.bin")
        },
        verify: |temp_dir| {
            let test_file = temp_dir.path().join("test.bin");
            let content = fs::read(&test_file).await?;
            assert_eq!(content, b"Hello");
        }
    }

    write_test! {
        name: test_write_all_bin_with_nulls,
        function: "fs::write_all_bin",
        content: r#"bytes:AAECqg=="#,
        setup: |temp_dir| {
            temp_dir.path().join("binary.bin")
        },
        verify: |temp_dir| {
            let test_file = temp_dir.path().join("binary.bin");
            let content = fs::read(&test_file).await?;
            assert_eq!(content, b"\x00\x01\x02\xaa");
        }
    }

    write_test! {
        name: test_write_all_bin_overwrite,
        function: "fs::write_all_bin",
        content: r#"bytes:AQI="#,
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("overwrite.bin");
            fs::write(&test_file, b"\x00\x00\x00\x00").await?;
            test_file
        },
        verify: |temp_dir| {
            let test_file = temp_dir.path().join("overwrite.bin");
            let content = fs::read(&test_file).await?;
            assert_eq!(content, b"\x01\x02");
        }
    }

    write_test! {
        name: test_write_all_invalid_path,
        function: "fs::write_all",
        content: r#""content""#,
        setup: |temp_dir| {
            temp_dir.path().join("nonexistent_dir/test.txt")
        },
        expect_error: true
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::test::init;
    use anyhow::Result;
    use graphix_rt::GXEvent;
    use netidx::subscriber::Value;
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
        let code = format!(r#"fs::watch(#interest: [`Modify], "{}")"#, watch_path);
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
        let code = format!(
            r#"fs::write_all_bin(#path: "{}", bytes:SGVsbG8=)"#,
            test_file.display()
        );
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
}
