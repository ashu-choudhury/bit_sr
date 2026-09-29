//! `.bsp` (Bit Screen-reader Package) archive handling, packaging, and extraction.
//!
//! Handles packaging and unpacking `.bsp` ZIP archives containing `manifest.toml`,
//! `plugin.wasm`, and optional static `assets/`.

use crate::manifest::{ManifestError, PluginManifest};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Cursor, Read, Seek, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// Errors encountered during `.bsp` packaging, reading, or extraction.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("Missing required file in .bsp archive: {0}")]
    MissingRequiredFile(&'static str),
    #[error("Invalid package path: {0}")]
    InvalidPath(String),
}

/// In-memory representation of an unpacked `.bsp` extension package.
#[derive(Debug, Clone)]
pub struct PluginPackage {
    /// Parsed and validated manifest.
    pub manifest: PluginManifest,
    /// Raw WebAssembly bytecode.
    pub wasm_bytes: Vec<u8>,
    /// Optional static assets indexed by relative path (e.g. "assets/chime.wav").
    pub assets: HashMap<String, Vec<u8>>,
}

impl PluginPackage {
    /// Creates a new in-memory plugin package.
    pub fn new(
        manifest: PluginManifest,
        wasm_bytes: Vec<u8>,
        assets: Option<HashMap<String, Vec<u8>>>,
    ) -> Self {
        Self {
            manifest,
            wasm_bytes,
            assets: assets.unwrap_or_default(),
        }
    }

    /// Bundles the package into a `.bsp` ZIP archive byte buffer.
    pub fn create_archive(&self) -> Result<Vec<u8>, PackageError> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buffer);
            let options = SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            // 1. Write manifest.toml
            zip.start_file("manifest.toml", options)?;
            let manifest_str = self.manifest.to_toml_string()?;
            zip.write_all(manifest_str.as_bytes())?;

            // 2. Write plugin.wasm
            zip.start_file("plugin.wasm", options)?;
            zip.write_all(&self.wasm_bytes)?;

            // 3. Write optional assets
            for (rel_path, data) in &self.assets {
                let clean_path = rel_path.trim_start_matches(['/', '\\']);
                zip.start_file(clean_path, options)?;
                zip.write_all(data)?;
            }

            zip.finish()?;
        }
        Ok(buffer.into_inner())
    }

    /// Saves the package as a `.bsp` file on disk.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<(), PackageError> {
        let bytes = self.create_archive()?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
        Ok(())
    }

    /// Reads and unpacks a package from any seekable reader.
    pub fn from_reader<R: Read + Seek>(reader: R) -> Result<Self, PackageError> {
        let mut archive = ZipArchive::new(reader)?;

        let mut manifest_opt = None;
        let mut wasm_bytes_opt = None;
        let mut assets = HashMap::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            if file.is_dir() {
                continue;
            }

            let mut contents = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut contents)?;

            if name == "manifest.toml" {
                let toml_str = String::from_utf8(contents)
                    .map_err(|e| PackageError::InvalidPath(format!("Invalid UTF-8 in manifest.toml: {e}")))?;
                manifest_opt = Some(PluginManifest::from_toml_str(&toml_str)?);
            } else if name == "plugin.wasm" {
                wasm_bytes_opt = Some(contents);
            } else if name.starts_with("assets/") || name.starts_with("assets\\") {
                assets.insert(name, contents);
            }
        }

        let manifest = manifest_opt.ok_or(PackageError::MissingRequiredFile("manifest.toml"))?;
        let wasm_bytes = wasm_bytes_opt.ok_or(PackageError::MissingRequiredFile("plugin.wasm"))?;

        Ok(Self {
            manifest,
            wasm_bytes,
            assets,
        })
    }

    /// Reads and unpacks a `.bsp` package directly from a byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PackageError> {
        Self::from_reader(Cursor::new(bytes))
    }

    /// Loads a package from a `.bsp` file on disk.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, PackageError> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }

    /// Extracts the package contents (manifest, wasm, and assets) into a directory on disk.
    pub fn extract_to_dir(&self, target_dir: impl AsRef<Path>) -> Result<(), PackageError> {
        let target = target_dir.as_ref();
        fs::create_dir_all(target)?;

        // Write manifest.toml
        let manifest_path = target.join("manifest.toml");
        fs::write(manifest_path, self.manifest.to_toml_string()?)?;

        // Write plugin.wasm
        let wasm_path = target.join("plugin.wasm");
        fs::write(wasm_path, &self.wasm_bytes)?;

        // Write assets
        for (rel_path, data) in &self.assets {
            let asset_path = target.join(rel_path);
            if let Some(parent) = asset_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(asset_path, data)?;
        }

        Ok(())
    }

    /// Loads an unpacked extension package directly from a directory.
    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self, PackageError> {
        let base = dir.as_ref();
        let manifest_path = base.join("manifest.toml");
        if !manifest_path.exists() {
            return Err(PackageError::MissingRequiredFile("manifest.toml"));
        }
        let manifest_str = fs::read_to_string(manifest_path)?;
        let manifest = PluginManifest::from_toml_str(&manifest_str)?;

        let wasm_path = base.join("plugin.wasm");
        if !wasm_path.exists() {
            return Err(PackageError::MissingRequiredFile("plugin.wasm"));
        }
        let wasm_bytes = fs::read(wasm_path)?;

        let mut assets = HashMap::new();
        let assets_dir = base.join("assets");
        if assets_dir.exists() && assets_dir.is_dir() {
            collect_assets_recursive(&assets_dir, &assets_dir, &mut assets)?;
        }

        Ok(Self {
            manifest,
            wasm_bytes,
            assets,
        })
    }
}

fn collect_assets_recursive(
    root: &Path,
    current: &Path,
    assets: &mut HashMap<String, Vec<u8>>,
) -> io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_assets_recursive(root, &path, assets)?;
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = format!("assets/{}", rel.to_string_lossy().replace('\\', "/"));
                let data = fs::read(&path)?;
                assets.insert(rel_str, data);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bsp_roundtrip_in_memory() {
        let toml_str = r#"
[plugin]
id = "org.bitsr.test_pkg"
name = "Test Package"
version = "1.0.0"
author = "Dev"
description = "Test Description"

[permissions]
speech = { output = true, filter = false }
"#;
        let manifest = PluginManifest::from_toml_str(toml_str).unwrap();
        let dummy_wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]; // Wasm magic bytes
        let mut assets = HashMap::new();
        assets.insert("assets/ding.wav".to_string(), b"RIFFdummydata".to_vec());

        let pkg = PluginPackage::new(manifest, dummy_wasm.clone(), Some(assets));
        let bsp_bytes = pkg.create_archive().expect("Archived successfully");

        let loaded = PluginPackage::from_bytes(&bsp_bytes).expect("Loaded from bytes");
        assert_eq!(loaded.manifest.plugin.id, "org.bitsr.test_pkg");
        assert_eq!(loaded.wasm_bytes, dummy_wasm);
        assert_eq!(loaded.assets.len(), 1);
        assert_eq!(loaded.assets.get("assets/ding.wav"), Some(&b"RIFFdummydata".to_vec()));
    }
}
