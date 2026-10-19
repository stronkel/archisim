//! buildingSMART's IDS test cases: each `pass-*.ids` must pass on its
//! `.ifc`, each `fail-*` and `invalid-*` must not.
//!
//! The cases are not in this repository (their licence does not allow
//! modifying them, and they are only test data). Download them with
//! `scripts/ids_testcases.sh` and run
//! `IDS_TESTCASES=<folder> cargo test -p archisim-ids --test conformance -- --nocapture`.
//! Without `IDS_TESTCASES` the test does nothing.

use std::path::Path;

/// Cases this checker knowingly does not meet, with the reason.
const KNOWN: &[(&str, &str)] = &[];

#[test]
fn buildingsmart_test_cases() {
    let Ok(dir) = std::env::var("IDS_TESTCASES") else {
        eprintln!("IDS_TESTCASES no está definido: se omiten los casos de buildingSMART");
        return;
    };
    let mut total = 0;
    let mut wrong = Vec::new();
    let mut dirs: Vec<_> = std::fs::read_dir(&dir)
        .expect("carpeta de casos")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for d in dirs {
        let mut files: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "ids"))
            .collect();
        files.sort();
        for ids_path in files {
            let name = ids_path.file_stem().unwrap().to_string_lossy().to_string();
            let ifc_path = ids_path.with_extension("ifc");
            if !Path::new(&ifc_path).exists() {
                continue;
            }
            total += 1;
            let expect_pass = name.starts_with("pass-");
            let ids = std::fs::read_to_string(&ids_path).unwrap();
            let ifc = std::fs::read_to_string(&ifc_path).unwrap();
            let passed = match archisim_ids::check_texts(&ids, &ifc) {
                Ok(r) => r.passed(),
                Err(_) => false,
            };
            let key = format!("{}/{}", d.file_name().unwrap().to_string_lossy(), name);
            if passed != expect_pass && !KNOWN.iter().any(|(k, _)| *k == key) {
                wrong.push(key);
            }
        }
    }
    eprintln!(
        "Casos de buildingSMART: {} de {total} correctos",
        total - wrong.len()
    );
    for w in &wrong {
        eprintln!("  ✘ {w}");
    }
    assert!(total > 0, "no hay casos en {dir}");
    assert!(wrong.is_empty(), "{} casos no cumplen", wrong.len());
}
