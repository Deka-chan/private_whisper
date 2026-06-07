use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Wheels whose bundled *.dll files make up the Windows CUDA-13 GPU runtime
/// (Microsoft onnxruntime-gpu + NVIDIA CUDA 13 / cuDNN 9). Downloaded and the
/// DLLs extracted on first run.
const RUNTIME_WHEELS: &[&str] = &[
    "https://aiinfra.pkgs.visualstudio.com/2692857e-05ef-43b4-ba9c-ccf1c22c437c/_packaging/62a612ee-4dee-42bc-8205-15b672eb4e4b/pypi/download/onnxruntime-gpu/1.24.1/onnxruntime_gpu-1.24.1-cp314-cp314-win_amd64.whl",
    "https://pypi.nvidia.com/nvidia-cuda-runtime/nvidia_cuda_runtime-13.3.29-py3-none-win_amd64.whl",
    "https://pypi.nvidia.com/nvidia-cublas/nvidia_cublas-13.5.1.27-py3-none-win_amd64.whl",
    "https://pypi.nvidia.com/nvidia-curand/nvidia_curand-10.4.3.29-py3-none-win_amd64.whl",
    "https://pypi.nvidia.com/nvidia-cufft/nvidia_cufft-12.3.0.29-py3-none-win_amd64.whl",
    "https://pypi.nvidia.com/nvidia-cuda-nvrtc/nvidia_cuda_nvrtc-13.3.33-py3-none-win_amd64.whl",
    "https://pypi.nvidia.com/nvidia-cudnn-cu13/nvidia_cudnn_cu13-9.23.0.39-py3-none-win_amd64.whl",
];

const ORT_DLL: &str = "onnxruntime.dll";
const MARKER: &str = ".runtime-complete";

pub fn dylib_path(dir: &Path) -> PathBuf {
    dir.join(ORT_DLL)
}

pub fn is_ready(dir: &Path) -> bool {
    dir.join(MARKER).exists() && dylib_path(dir).exists()
}

/// progress(wheel_index, wheels_total, bytes_done_this_wheel, bytes_total_this_wheel)
pub type ProgressFn<'a> = dyn FnMut(usize, usize, u64, u64) + 'a;

/// Ensure the GPU runtime DLLs are present in `dir`; returns the path to
/// onnxruntime.dll. Idempotent (skips when already complete).
pub fn ensure(dir: &Path, progress: &mut ProgressFn) -> anyhow::Result<PathBuf> {
    if is_ready(dir) {
        return Ok(dylib_path(dir));
    }
    std::fs::create_dir_all(dir)?;
    let total = RUNTIME_WHEELS.len();
    for (i, url) in RUNTIME_WHEELS.iter().enumerate() {
        let resp = reqwest::blocking::get(*url)?.error_for_status()?;
        let len = resp.content_length().unwrap_or(0);
        let tmp = dir.join(format!("wheel-{i}.whl.part"));
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
        extract_dlls(&tmp, dir)?;
        std::fs::remove_file(&tmp).ok();
    }
    if !dylib_path(dir).exists() {
        anyhow::bail!("runtime incomplete: {} not found after extraction", ORT_DLL);
    }
    std::fs::write(dir.join(MARKER), b"ok")?;
    Ok(dylib_path(dir))
}

/// Extract every *.dll entry from a wheel (zip), flattened to its basename, into
/// `dir`. Returns the count written.
pub fn extract_dlls(wheel: &Path, dir: &Path) -> anyhow::Result<usize> {
    let file = std::fs::File::open(wheel)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut count = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.to_ascii_lowercase().ends_with(".dll") {
            let base = name.rsplit('/').next().unwrap_or(name.as_str());
            let mut f = std::fs::File::create(dir.join(base))?;
            std::io::copy(&mut entry, &mut f)?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extract_dlls_flattens_dll_entries_and_ignores_other_files() {
        let temp = tempfile::tempdir().unwrap();
        let wheel = temp.path().join("runtime.whl");

        {
            let file = std::fs::File::create(&wheel).unwrap();
            let mut writer = zip::write::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            writer.start_file("nvidia/foo/bar.dll", options).unwrap();
            writer.write_all(b"dll bytes").unwrap();
            writer.start_file("nvidia/foo/readme.txt", options).unwrap();
            writer.write_all(b"readme").unwrap();
            writer.finish().unwrap();
        }

        let out_dir = temp.path().join("runtime");
        std::fs::create_dir(&out_dir).unwrap();

        let count = extract_dlls(&wheel, &out_dir).unwrap();

        assert_eq!(count, 1);
        assert_eq!(
            std::fs::read(out_dir.join("bar.dll")).unwrap(),
            b"dll bytes"
        );
        assert!(!out_dir.join("readme.txt").exists());
    }
}
