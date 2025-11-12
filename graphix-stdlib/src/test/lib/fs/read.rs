use anyhow::{bail, Result};
use arcstr::ArcStr;
use netidx::subscriber::Value;
use tokio::fs;
use tokio::time::Duration;

use crate::test::init;
use graphix_rt::GXEvent;
use poolshark::global::GPooled;
use tokio::sync::mpsc;

/// Macro to create fs::read_* tests with common setup/teardown logic
macro_rules! read_test {
    // Error expectation case - delegates to main pattern
    (
        name: $test_name:ident,
        function: $func:expr,
        setup: |$temp_dir:ident| $setup:block,
        expect_error
    ) => {
        read_test! {
            name: $test_name,
            function: $func,
            setup: |$temp_dir| $setup,
            expect: |v: Value| -> Result<()> {
                if matches!(v, Value::Error(_)) {
                    Ok(())
                } else {
                    panic!("expected Error value, got: {v:?}")
                }
            }
        }
    };
    // Main pattern with custom expectation
    (
        name: $test_name:ident,
        function: $func:expr,
        setup: |$temp_dir:ident| $setup:block,
        expect: $expect_payload:expr
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
    expect: |v: Value| -> Result<()> {
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
    expect_error
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
    expect: |v: Value| -> Result<()> {
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
    expect: |v: Value| -> Result<()> {
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
    expect_error
}
