use crate::run;
use anyhow::{bail, Result};
use netidx::subscriber::Value;
use std::path::Path;

// Test basic tempdir creation with null trigger
const TEMPDIR_BASIC: &str = r#"fs::tempdir(null)"#;

run!(test_tempdir_basic, TEMPDIR_BASIC, |v: Result<&Value>| {
    match v {
        Ok(Value::String(path)) => {
            let p = Path::new(&**path);
            p.exists() && p.is_dir()
        }
        _ => false,
    }
});

// Test tempdir creation with explicit parent directory
const TEMPDIR_WITH_IN: &str = r#"{
  let parent = fs::tempdir(null)?;
  fs::tempdir(#in: parent, null)
}"#;

run!(test_tempdir_with_in, TEMPDIR_WITH_IN, |v: Result<&Value>| {
    match v {
        Ok(Value::String(path)) => {
            let p = Path::new(&**path);
            p.exists() && p.is_dir()
        }
        _ => false,
    }
});

// Test tempdir with prefix
const TEMPDIR_WITH_PREFIX: &str = r#"fs::tempdir(#name: `Prefix("myprefix_"), null)"#;

run!(test_tempdir_with_prefix, TEMPDIR_WITH_PREFIX, |v: Result<&Value>| {
    match v {
        Ok(Value::String(path)) => {
            let p = Path::new(&**path);
            if !p.exists() || !p.is_dir() {
                return false;
            }
            // Check that the directory name starts with our prefix
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                name.starts_with("myprefix_")
            } else {
                false
            }
        }
        _ => false,
    }
});

// Test tempdir with suffix
const TEMPDIR_WITH_SUFFIX: &str = r#"fs::tempdir(#name: `Suffix("_mysuffix"), null)"#;

run!(test_tempdir_with_suffix, TEMPDIR_WITH_SUFFIX, |v: Result<&Value>| {
    match v {
        Ok(Value::String(path)) => {
            let p = Path::new(&**path);
            if !p.exists() || !p.is_dir() {
                return false;
            }
            // Check that the directory name ends with our suffix
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                name.ends_with("_mysuffix")
            } else {
                false
            }
        }
        _ => false,
    }
});

// Test tempdir with both parent dir and prefix
const TEMPDIR_WITH_IN_AND_PREFIX: &str = r#"{
  let parent = fs::tempdir(null)?;
  fs::tempdir(#in: parent, #name: `Prefix("test_"), null)
}"#;

run!(
    test_tempdir_with_in_and_prefix,
    TEMPDIR_WITH_IN_AND_PREFIX,
    |v: Result<&Value>| {
        match v {
            Ok(Value::String(path)) => {
                let p = Path::new(&**path);
                if !p.exists() || !p.is_dir() {
                    return false;
                }
                // Check prefix
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    name.starts_with("test_")
                } else {
                    false
                }
            }
            _ => false,
        }
    }
);

// Test tempdir with both parent dir and suffix
const TEMPDIR_WITH_IN_AND_SUFFIX: &str = r#"{
  let parent = fs::tempdir(null)?;
  fs::tempdir(#in: parent, #name: `Suffix("_test"), null)
}"#;

run!(
    test_tempdir_with_in_and_suffix,
    TEMPDIR_WITH_IN_AND_SUFFIX,
    |v: Result<&Value>| {
        match v {
            Ok(Value::String(path)) => {
                let p = Path::new(&**path);
                if !p.exists() || !p.is_dir() {
                    return false;
                }
                // Check suffix
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    name.ends_with("_test")
                } else {
                    false
                }
            }
            _ => false,
        }
    }
);

// Note: Tests for tempdir trigger updates are not included here because
// they would require mutable state management which is not easily testable
// in the current test framework. The trigger functionality is tested
// indirectly through the integration tests.

// Test tempdir error handling with invalid parent directory
const TEMPDIR_INVALID_PARENT: &str =
    r#"fs::tempdir(#in: "/this/path/should/not/exist/anywhere", null)"#;

run!(
    test_tempdir_invalid_parent,
    TEMPDIR_INVALID_PARENT,
    |v: Result<&Value>| { matches!(v, Ok(Value::Error(_))) }
);

// Test using tempdir for write/read cycle
const TEMPDIR_WRITE_READ_CYCLE: &str = r#"{
  let temp = fs::tempdir(null)?;
  let file_path = fs::join_path(temp, "cycle_test.txt");
  let write_result = fs::write_all(#path: file_path, "Hello from tempdir!");
  fs::read_all(write_result ~ file_path)
}"#;

run!(
    test_tempdir_write_read_cycle,
    TEMPDIR_WRITE_READ_CYCLE,
    |v: Result<&Value>| {
        matches!(v, Ok(Value::String(s)) if &**s == "Hello from tempdir!")
    }
);
