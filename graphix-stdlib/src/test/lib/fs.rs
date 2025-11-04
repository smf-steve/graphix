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
