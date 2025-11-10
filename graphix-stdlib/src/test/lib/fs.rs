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

    /// Macro to create fs::watch tests with common setup/teardown logic.
    /// Supports both simple single-action tests and complex multi-event sequences.
    macro_rules! watch_test {
        // Simple pattern: single action after establishment, boolean expectation
        // This delegates to the complex pattern with sensible defaults
        (
            name: $test_name:ident,
            interest: $interest:expr,
            setup: |$temp_dir:ident| $setup:block,
            action: |$action_dir:ident| $action:block,
            expect: $expect:expr
        ) => {
            watch_test! {
                name: $test_name,
                interest: $interest,
                timeout_secs: 2,
                setup: |$temp_dir| {
                    $setup
                    $temp_dir.path()
                },
                state: {
                    _event_count: usize = 0,
                },
                on_event: |count, temp_dir, _event_count| {
                    *_event_count = count;
                    if count == 1 {
                        eprintln!("watch established, performing action");
                        let $action_dir = &temp_dir;
                        $action
                    }
                },
                verify: {
                    let got_event = _event_count > 1;
                    assert_eq!(got_event, $expect,
                        "Expected event: {}, Got event: {}", $expect, got_event)
                }
            }
        };

        // Complex pattern: multi-event sequence with state tracking
        (
            name: $test_name:ident,
            interest: $interest:expr,
            timeout_secs: $timeout:expr,
            setup: |$setup_dir:ident| $setup:block,
            state: { $($state_name:ident: $state_type:ty = $state_init:expr),* $(,)? },
            on_event: |$event_count:ident, $event_dir:ident, $($state_param:ident),*| $on_event:block,
            verify: { $($verify:stmt);* $(;)? }
        ) => {
            #[tokio::test(flavor = "current_thread")]
            async fn $test_name() -> Result<()> {
                let (tx, mut rx) = mpsc::channel::<GPooled<Vec<GXEvent<_>>>>(10);
                let ctx = init(tx).await?;
                let temp_dir = tempfile::tempdir()?;

                // Run setup
                let watch_path = {
                    let $setup_dir = &temp_dir;
                    $setup
                };

                // Start watching
                let code = format!(
                    r#"fs::watch(#interest: {}, "{}")"#,
                    $interest, watch_path.display()
                );

                let compiled = ctx.rt.compile(ArcStr::from(code)).await?;
                let eid = compiled.exprs[0].id;

                let timeout = tokio::time::sleep(Duration::from_secs($timeout));
                tokio::pin!(timeout);
                let mut event_count = 0;
                $(let mut $state_name: $state_type = $state_init;)*

                loop {
                    tokio::select! {
                        _ = &mut timeout => break,
                        Some(mut batch) = rx.recv() => {
                            for event in batch.drain(..) {
                                if let GXEvent::Updated(id, v) = event {
                                    if id == eid && matches!(v, Value::String(_)) {
                                        event_count += 1;
                                        eprintln!("Event #{event_count}: {v}");

                                        let $event_count = event_count;
                                        let $event_dir = &temp_dir;
                                        $(let $state_param = &mut $state_param;)*
                                        $on_event
                                    }
                                }
                            }
                        }
                    }
                }

                $($verify;)*
                Ok(())
            }
        };
    }

    // Test file creation detection
    watch_test! {
        name: test_watch_create_file,
        interest: "[`Established, `Create]",
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
        interest: "[`Established, `Modify]",
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
        interest: "[`Established, `Delete]",
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
        interest: "[`Established, `Create]",
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
    watch_test! {
        name: test_watch_nonexistent_file_created,
        interest: "[`Create, `Established]",
        timeout_secs: 5,
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("nonexistent.txt");
            let _ = std::fs::remove_file(&test_file);
            test_file
        },
        state: {
            file_created: bool = false,
            got_create: bool = false,
        },
        on_event: |count, temp_dir, file_created, got_create| {
            let test_file = temp_dir.path().join("nonexistent.txt");
            if count == 1 && !*file_created {
                eprintln!("Creating file");
                fs::write(&test_file, b"hello").await?;
                *file_created = true;
            } else if count >= 2 {
                *got_create = true;
            }
        },
        verify: {
            assert!(got_create, "Did not receive create event for non-existent file");
        }
    }

    // Test watching existing file, deleting it, then recreating it
    watch_test! {
        name: test_watch_delete_then_recreate,
        interest: "[`Established, `Create, `Delete, `Modify]",
        timeout_secs: 6,
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("delete_recreate.txt");
            fs::write(&test_file, b"initial").await?;
            test_file
        },
        state: {
            got_delete: bool = false,
            got_create: bool = false,
        },
        on_event: |count, temp_dir, got_delete, got_create| {
            let test_file = temp_dir.path().join("delete_recreate.txt");
            if count == 1 {
                eprintln!("Deleting file");
                fs::remove_file(&test_file).await?;
            } else if count == 2 {
                *got_delete = true;
                eprintln!("Recreating file");
                tokio::time::sleep(Duration::from_millis(100)).await;
                fs::write(&test_file, b"recreated").await?;
            } else if count == 3 {
                *got_create = true;
            }
        },
        verify: {
            assert!(got_delete, "Did not receive delete event");
            assert!(got_create, "Did not receive create event after recreation");
        }
    }

    // Test renaming parent directory
    watch_test! {
        name: test_watch_parent_rename,
        interest: "[`Established, `Delete, `Create]",
        timeout_secs: 4,
        setup: |temp_dir| {
            let parent_dir = temp_dir.path().join("parent");
            fs::create_dir(&parent_dir).await?;
            let test_file = parent_dir.join("file.txt");
            fs::write(&test_file, b"content").await?;
            test_file
        },
        state: {
            got_delete: bool = false,
        },
        on_event: |count, temp_dir, got_delete| {
            if count == 1 {
                let parent_dir = temp_dir.path().join("parent");
                let new_parent = temp_dir.path().join("parent_renamed");
                eprintln!("Renaming parent directory");
                fs::rename(&parent_dir, &new_parent).await?;
            } else {
                *got_delete = true;
            }
        },
        verify: {
            assert!(got_delete, "Did not receive delete event after parent rename");
        }
    }

    // Test multi-level parent creation
    watch_test! {
        name: test_watch_multilevel_parent_creation,
        interest: "[`Established, `Create]",
        timeout_secs: 5,
        setup: |temp_dir| {
            let deep_file = temp_dir.path().join("a/b/c/file.txt");
            let _ = fs::remove_dir_all(temp_dir.path().join("a")).await;
            deep_file
        },
        state: {
            got_create: bool = false,
        },
        on_event: |count, temp_dir, got_create| {
            let deep_file = temp_dir.path().join("a/b/c/file.txt");
            if count == 1 {
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
                *got_create = true;
            }
        },
        verify: {
            assert!(got_create, "Did not receive create event for deep file");
        }
    }

    // Test deep parent rename (rename two levels up)
    watch_test! {
        name: test_watch_deep_parent_rename,
        interest: "[`Established, `Delete]",
        timeout_secs: 4,
        setup: |temp_dir| {
            let a = temp_dir.path().join("a");
            let b = a.join("b");
            let c = b.join("c");
            let d = c.join("d");
            fs::create_dir_all(&d).await?;
            let test_file = d.join("file.txt");
            fs::write(&test_file, b"content").await?;
            test_file
        },
        state: {
            got_delete: bool = false,
        },
        on_event: |count, temp_dir, got_delete| {
            if count == 1 {
                let a = temp_dir.path().join("a");
                let b = a.join("b");
                let b2 = a.join("b2");
                eprintln!("Renaming /a/b to /a/b2 (two levels up)");
                fs::rename(&b, &b2).await?;
            } else {
                *got_delete = true;
            }
        },
        verify: {
            assert!(got_delete, "Did not receive delete event after deep parent rename");
        }
    }

    // Test race with parent deletion
    watch_test! {
        name: test_watch_parent_tree_deletion,
        interest: "[`Established, `Delete]",
        timeout_secs: 4,
        setup: |temp_dir| {
            let a = temp_dir.path().join("a");
            let b = a.join("b");
            fs::create_dir_all(&b).await?;
            let test_file = b.join("file.txt");
            fs::write(&test_file, b"content").await?;
            test_file
        },
        state: {
            got_delete: bool = false,
        },
        on_event: |count, temp_dir, got_delete| {
            if count == 1 {
                let a = temp_dir.path().join("a");
                eprintln!("Deleting entire /a directory tree");
                fs::remove_dir_all(&a).await?;
            } else {
                *got_delete = true;
            }
        },
        verify: {
            assert!(got_delete, "Did not receive delete event after parent tree deletion");
        }
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
        let code1 = format!(
            r#"fs::watch(#interest: [`Established, `Create], "{}")"#,
            file1.display()
        );
        let code2 = format!(
            r#"fs::watch(#interest: [`Established, `Create], "{}")"#,
            file2.display()
        );

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
    watch_test! {
        name: test_watch_established_to_pending,
        interest: "[`Delete, `Established]",
        timeout_secs: 4,
        setup: |temp_dir| {
            let subdir = temp_dir.path().join("subdir");
            fs::create_dir(&subdir).await?;
            let test_file = subdir.join("file.txt");
            fs::write(&test_file, b"content").await?;
            test_file
        },
        state: {
            got_established: bool = false,
            got_delete: bool = false,
        },
        on_event: |count, temp_dir, got_established, got_delete| {
            if count == 1 {
                *got_established = true;
                eprintln!("Deleting parent directory");
                let subdir = temp_dir.path().join("subdir");
                fs::remove_dir_all(&subdir).await?;
            } else {
                *got_delete = true;
            }
        },
        verify: {
            assert!(got_established, "Did not receive Established event");
            assert!(got_delete, "Did not receive Delete event after parent deletion");
        }
    }

    // Test file -> directory transition
    watch_test! {
        name: test_watch_file_to_directory,
        interest: "[`Established, `Delete, `Create]",
        timeout_secs: 4,
        setup: |temp_dir| {
            let path = temp_dir.path().join("transform");
            fs::write(&path, b"file content").await?;
            path
        },
        state: {
            got_delete: bool = false,
            got_create: bool = false,
        },
        on_event: |count, temp_dir, got_delete, got_create| {
            let path = temp_dir.path().join("transform");
            if count == 1 {
                eprintln!("Deleting file and creating directory");
                fs::remove_file(&path).await?;
                tokio::time::sleep(Duration::from_millis(100)).await;
                fs::create_dir(&path).await?;
            } else if count == 2 {
                *got_delete = true;
            } else if count == 3 {
                *got_create = true;
            }
        },
        verify: {
            assert!(got_delete, "Did not receive delete event");
            assert!(got_create, "Did not receive create event for directory");
        }
    }

    // Test symlink with non-existent target
    watch_test! {
        name: test_watch_symlink_nonexistent_target,
        interest: "[`Established, `Create, `Modify]",
        timeout_secs: 4,
        setup: |temp_dir| {
            use std::os::unix::fs::symlink;
            let target = temp_dir.path().join("target.txt");
            let link = temp_dir.path().join("link.txt");
            symlink(&target, &link).unwrap();
            link
        },
        state: {
            got_event: bool = false,
        },
        on_event: |count, temp_dir, got_event| {
            let target = temp_dir.path().join("target.txt");
            if count == 1 {
                eprintln!("Creating symlink target");
                fs::write(&target, b"target content").await?;
            } else {
                *got_event = true;
            }
        },
        verify: {
            assert!(got_event, "Did not receive event when symlink target was created");
        }
    }

    // Test deleting and recreating symlink target (watches resolve through symlinks)
    watch_test! {
        name: test_watch_symlink_recreate,
        interest: "[`Established, `Delete, `Create]",
        timeout_secs: 4,
        setup: |temp_dir| {
            use std::os::unix::fs::symlink;
            let target = temp_dir.path().join("target.txt");
            let link = temp_dir.path().join("link.txt");
            fs::write(&target, b"content").await?;
            symlink(&target, &link)?;
            link
        },
        state: {
            got_delete: bool = false,
            got_create: bool = false,
        },
        on_event: |count, temp_dir, got_delete, got_create| {
            let target = temp_dir.path().join("target.txt");
            if count == 1 {
                eprintln!("Deleting target");
                fs::remove_file(&target).await?;
                tokio::time::sleep(Duration::from_millis(500)).await;
                eprintln!("Recreating target");
                fs::write(&target, b"content").await?;
            } else if count == 2 {
                *got_delete = true;
            } else if count == 3 {
                *got_create = true;
            }
        },
        verify: {
            assert!(got_delete, "Did not receive delete event for target");
            assert!(got_create, "Did not receive create event for target");
        }
    }

    // Test rapid delete-recreate within poll interval
    watch_test! {
        name: test_watch_rapid_transitions,
        interest: "[`Established, `Delete, `Create, `Modify]",
        timeout_secs: 5,
        setup: |temp_dir| {
            let test_file = temp_dir.path().join("rapid.txt");
            fs::write(&test_file, b"v1").await?;
            test_file.clone()
        },
        state: {
            total_events: usize = 0,
            test_file_path: std::path::PathBuf = std::path::PathBuf::new(),
        },
        on_event: |count, temp_dir, total_events, test_file_path| {
            *total_events = count;
            if count == 1 {
                eprintln!("Performing rapid delete-recreate cycle");
                let test_file = temp_dir.path().join("rapid.txt");
                *test_file_path = test_file.clone();
                fs::remove_file(&test_file).await?;
                tokio::time::sleep(Duration::from_millis(10)).await;
                fs::write(&test_file, b"v2").await?;
            }
        },
        verify: {
            assert!(test_file_path.exists(), "File should exist after rapid transitions");
            let content = fs::read(&test_file_path).await?;
            assert_eq!(content, b"v2", "File should have final content");
            assert!(total_events >= 1, "Should get at least the initial event");
            eprintln!("Total events received: {total_events}");
        }
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
        let code = r#"fs::set_global_watch_parameters(#poll_batch_size: 200, #poll_interval: duration:100.ms)"#;
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
                                    r#"fs::watch(#interest: [`Established, `Create], "{}")"#,
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
