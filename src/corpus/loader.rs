use crate::corpus::sample::{Sample, Sidecar};
use std::fs;
use std::path::{Path, PathBuf};

pub fn load_corpus(root: &Path) -> Result<Vec<Sample>, String> {
    if !root.is_dir() {
        return Err(format!(
            "corpus root is not a directory: {}",
            root.display()
        ));
    }

    let mut samples: Vec<Sample> = Vec::new();

    for cohort_path in read_dir_sorted(root)? {
        if !cohort_path.is_dir() {
            continue;
        }
        for verdict_name in ["clean", "threat"] {
            let verdict_path = cohort_path.join(verdict_name);
            if !verdict_path.is_dir() {
                continue;
            }
            for entry_path in read_dir_sorted(&verdict_path)? {
                if entry_path.extension().and_then(|s| s.to_str()) != Some("toml") {
                    continue;
                }
                let raw = fs::read_to_string(&entry_path)
                    .map_err(|e| format!("read sidecar {}: {}", entry_path.display(), e))?;
                let sidecar: Sidecar = toml::from_str(&raw)
                    .map_err(|e| format!("parse sidecar {}: {}", entry_path.display(), e))?;
                let text_full_path = verdict_path.join(&sidecar.text_path);
                samples.push(Sample {
                    sidecar,
                    text_full_path,
                    sidecar_path: entry_path,
                });
            }
        }
    }

    samples.sort_by(|a, b| {
        (a.sidecar.cohort.as_str(), a.sidecar.id.as_str())
            .cmp(&(b.sidecar.cohort.as_str(), b.sidecar.id.as_str()))
    });

    Ok(samples)
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| format!("read {}: {}", dir.display(), e))? {
        let entry = entry.map_err(|e| format!("entry in {}: {}", dir.display(), e))?;
        out.push(entry.path());
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::sample::{Format, Severity, Verdict};

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn loads_three_samples_in_cohort_id_order() {
        let samples = load_corpus(&fixture("loader-basic")).expect("load");
        let keys: Vec<(&str, &str)> = samples
            .iter()
            .map(|s| (s.sidecar.cohort.as_str(), s.sidecar.id.as_str()))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("cohort-a", "0001"),
                ("cohort-a", "0101"),
                ("cohort-b", "0002"),
            ]
        );
    }

    #[test]
    fn roundtrips_every_sidecar_field() {
        let samples = load_corpus(&fixture("loader-basic")).expect("load");
        let by_id = |id: &str| {
            samples
                .iter()
                .find(|s| s.sidecar.id == id)
                .unwrap_or_else(|| panic!("missing fixture sample {id}"))
        };

        let s1 = by_id("0001");
        assert_eq!(s1.sidecar.cohort, "cohort-a");
        assert_eq!(s1.sidecar.text_path, "0001.txt");
        assert_eq!(s1.sidecar.verdict, Verdict::Clean);
        assert_eq!(s1.sidecar.format, Format::RawText);
        assert_eq!(s1.sidecar.source, "handwritten");
        assert_eq!(s1.sidecar.license, "internal");
        assert!(s1.sidecar.expected_categories.is_empty());
        assert_eq!(s1.sidecar.expected_min_severity, None);
        assert_eq!(s1.sidecar.seed_id, None);
        assert_eq!(s1.sidecar.tags, vec!["english", "short"]);
        assert_eq!(s1.sidecar.notes, "first clean sample");

        let s101 = by_id("0101");
        assert_eq!(s101.sidecar.verdict, Verdict::Threat);
        assert_eq!(
            s101.sidecar.expected_categories,
            vec!["prompt_injection", "instruction_override"]
        );
        assert_eq!(s101.sidecar.expected_min_severity, Some(Severity::High));
        assert_eq!(
            s101.sidecar.notes,
            "classic ignore-previous-instructions lead-in"
        );

        let s2 = by_id("0002");
        assert_eq!(s2.sidecar.cohort, "cohort-b");
        assert_eq!(s2.sidecar.format, Format::Markdown);
        assert_eq!(s2.sidecar.seed_id, Some("0001".to_string()));
        assert_eq!(s2.sidecar.notes, "");
    }

    #[test]
    fn text_path_resolves_relative_to_sidecar_directory() {
        let samples = load_corpus(&fixture("loader-basic")).expect("load");
        for sample in &samples {
            let bytes = sample
                .read_bytes()
                .unwrap_or_else(|e| panic!("read {}: {e}", sample.sidecar.id));
            assert!(!bytes.is_empty(), "{} text empty", sample.sidecar.id);
        }
    }

    #[test]
    fn rejects_non_directory_root() {
        let err = load_corpus(Path::new("/definitely/not/a/real/path")).expect_err("must error");
        assert!(err.contains("not a directory"), "got: {err}");
    }
}
