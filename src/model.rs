use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct ModelFile {
    pub filename: &'static str,
    pub url: &'static str,
    pub expected_size: u64,
}

pub const MODEL_FILES: &[ModelFile] = &[
    ModelFile {
        filename: "encoder-model.onnx",
        url: "https://huggingface.co/grikdotnet/parakeet-tdt-0.6b-fp16/resolve/main/encoder-model.fp16.onnx",
        expected_size: 1_238_960_452,
    },
    ModelFile {
        filename: "decoder_joint-model.onnx",
        url: "https://huggingface.co/grikdotnet/parakeet-tdt-0.6b-fp16/resolve/main/decoder_joint-model.fp16.onnx",
        expected_size: 36_266_140,
    },
    ModelFile {
        filename: "vocab.txt",
        url: "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/vocab.txt",
        expected_size: 93_939,
    },
];

/// A file counts as present if it exists and (when expected_size > 0) matches size.
fn file_present(dir: &Path, f: &ModelFile) -> bool {
    let p = dir.join(f.filename);
    match std::fs::metadata(&p) {
        Ok(m) => f.expected_size == 0 || m.len() == f.expected_size,
        Err(_) => false,
    }
}

pub fn missing_files(dir: &Path) -> Vec<&'static str> {
    MODEL_FILES
        .iter()
        .filter(|f| !file_present(dir, f))
        .map(|f| f.filename)
        .collect()
}

pub fn is_complete(dir: &Path) -> bool {
    missing_files(dir).is_empty()
}

/// Progress callback: (file_index, files_total, bytes_done_this_file, bytes_total_this_file).
pub type ProgressFn<'a> = dyn FnMut(usize, usize, u64, u64) + 'a;

/// Download every missing file into `dir`, reporting progress. Idempotent:
/// files already present and size-matching are skipped.
pub fn ensure(dir: &Path, progress: &mut ProgressFn) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let total = MODEL_FILES.len();

    for (i, f) in MODEL_FILES.iter().enumerate() {
        if file_present(dir, f) {
            continue;
        }

        let resp = reqwest::blocking::get(f.url)?.error_for_status()?;
        let len = resp.content_length().unwrap_or(f.expected_size);
        let tmp = dir.join(format!("{}.part", f.filename));
        let mut out = std::fs::File::create(&tmp)?;
        let mut reader = resp;
        let mut buf = [0u8; 1 << 16];
        let mut done: u64 = 0;

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            done += n as u64;
            progress(i, total, done, len);
        }

        out.flush()?;
        drop(out);
        std::fs::rename(&tmp, dir.join(f.filename))?;
    }

    if !is_complete(dir) {
        anyhow::bail!(
            "model incomplete after download: missing {:?}",
            missing_files(dir)
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str, bytes: u64) {
        let file = std::fs::File::create(dir.join(name)).unwrap();
        file.set_len(bytes).unwrap();
    }

    #[test]
    fn empty_dir_is_incomplete_and_lists_all() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_complete(dir.path()));
        assert_eq!(missing_files(dir.path()).len(), MODEL_FILES.len());
    }

    #[test]
    fn all_present_is_complete() {
        let dir = tempfile::tempdir().unwrap();
        for f in MODEL_FILES {
            touch(dir.path(), f.filename, f.expected_size);
        }
        assert!(is_complete(dir.path()));
        assert!(missing_files(dir.path()).is_empty());
    }

    #[test]
    fn size_mismatch_counts_as_missing_when_expected_set() {
        let dir = tempfile::tempdir().unwrap();
        let f = ModelFile {
            filename: "x.bin",
            url: "",
            expected_size: 100,
        };
        touch(dir.path(), "x.bin", 10);
        assert!(!file_present(dir.path(), &f));
    }
}
