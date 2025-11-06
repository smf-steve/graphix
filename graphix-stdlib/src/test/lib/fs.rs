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
