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
