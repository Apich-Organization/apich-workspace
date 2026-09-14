use crate::error::Result;
use crate::error::SlideError;
use crate::model::SlideDeck;
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::path::Path;

/// Metadata header stored inside the `.slide` standalone package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlidePackageMetadata {
    pub format_version: u32,
    pub title: String,
    pub total_slides: usize,
    pub default_animation: String,
    pub compression: String,
    pub generator: String,
}

impl Default for SlidePackageMetadata {
    fn default() -> Self {
        Self {
            format_version: 1,
            title: "Presentation".to_string(),
            total_slides: 0,
            default_animation: "fade".to_string(),
            compression: "lzma2-max".to_string(),
            generator: format!("cargo-slide {}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Package a `SlideDeck` along with its optional asset directory into a standalone `.slide` file.
pub fn pack_deck_with_assets(
    deck: &SlideDeck,
    assets_dir: Option<&Path>,
    output_path: &Path,
) -> Result<()> {
    if let Some(parent) = output_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }

    let metadata = SlidePackageMetadata {
        format_version: 1,
        title: deck.title.clone(),
        total_slides: deck.total_slides(),
        default_animation: deck.default_animation.clone(),
        compression: "lzma2-max".to_string(),
        generator: format!("cargo-slide {}", env!("CARGO_PKG_VERSION")),
    };

    let metadata_json = serde_json::to_vec_pretty(&metadata)?;
    let deck_json = serde_json::to_vec(deck)?;

    // Create an in-memory TAR archive
    let mut tar_builder = tar::Builder::new(Vec::new());

    // Add manifest.json
    let mut meta_header = tar::Header::new_gnu();
    meta_header.set_size(metadata_json.len() as u64);
    meta_header.set_mode(0o644);
    meta_header.set_cksum();
    tar_builder.append_data(&mut meta_header, "manifest.json", &metadata_json[..])?;

    // Add deck.json
    let mut deck_header = tar::Header::new_gnu();
    deck_header.set_size(deck_json.len() as u64);
    deck_header.set_mode(0o644);
    deck_header.set_cksum();
    tar_builder.append_data(&mut deck_header, "deck.json", &deck_json[..])?;

    // Bundle any assets from base directory and any local files linked across slides
    let mut added_paths = std::collections::HashSet::<String>::new();
    added_paths.insert("manifest.json".to_string());
    added_paths.insert("deck.json".to_string());

    if let Some(base) = assets_dir {
        let assets_folder = base.join("assets");
        if assets_folder.is_dir() {
            for entry in walkdir::WalkDir::new(&assets_folder).into_iter().flatten() {
                if entry.file_type().is_file() {
                    let file_path = entry.path();
                    if let Ok(rel) = file_path.strip_prefix(base) {
                        let rel_str = rel.to_string_lossy().replace('\\', "/");
                        if added_paths.insert(rel_str.clone())
                            && let Ok(data) = std::fs::read(file_path)
                        {
                            let mut header = tar::Header::new_gnu();
                            header.set_size(data.len() as u64);
                            header.set_mode(0o644);
                            header.set_cksum();
                            let _ = tar_builder.append_data(&mut header, &rel_str, &data[..]);
                        }
                    }
                }
            }
        }

        // Scan hotspots across all slides for local relative file links, audio, and video
        for slide in &deck.slides {
            for hotspot in &slide.hotspots {
                let candidate_path = match hotspot {
                    | crate::model::Hotspot::Link { target, .. } => {
                        let clean = target.strip_prefix('#').unwrap_or(target);
                        if clean.starts_with("chart:")
                            || clean.starts_with("video:")
                            || clean.starts_with("audio:")
                            || clean.starts_with("step:")
                            || clean.starts_with("transition:")
                            || clean.starts_with("mailto:")
                            || clean.contains("://")
                            || target.starts_with('#')
                        {
                            None
                        } else {
                            Some(target.strip_prefix("file://").unwrap_or(target))
                        }
                    },
                    | crate::model::Hotspot::Video { source, .. }
                    | crate::model::Hotspot::Audio { source, .. } => {
                        if source.contains("://") || source.starts_with('#') {
                            None
                        } else {
                            Some(source.strip_prefix("file://").unwrap_or(source))
                        }
                    },
                    | _ => None,
                };

                if let Some(raw_rel) = candidate_path {
                    let rel_clean = raw_rel.replace('\\', "/");
                    let file_path = base.join(&rel_clean);
                    if file_path.is_file()
                        && added_paths.insert(rel_clean.clone())
                        && let Ok(data) = std::fs::read(&file_path)
                    {
                        let mut header = tar::Header::new_gnu();
                        header.set_size(data.len() as u64);
                        header.set_mode(0o644);
                        header.set_cksum();
                        let _ = tar_builder.append_data(&mut header, &rel_clean, &data[..]);
                    }
                }
            }
        }
    }

    let uncompressed_tar = tar_builder.into_inner()?;

    // Compress with LZMA2 at maximum compression ratio (Level 9 + Extreme preset)
    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);

    // XZ container format with LZMA2 filter preset 9 | Extreme flag
    const LZMA_PRESET_EXTREME: u32 = 1 << 31;
    let mut encoder =
        xz2::write::XzEncoder::new_stream(
            writer,
            xz2::stream::Stream::new_easy_encoder(
                9 | LZMA_PRESET_EXTREME,
                xz2::stream::Check::Crc64,
            )
            .map_err(|e| {
                SlideError::Compilation(format!("Failed to initialize LZMA2 encoder: {e}"))
            })?,
        );

    encoder.write_all(&uncompressed_tar)?;
    let mut writer = encoder
        .finish()
        .map_err(|e| SlideError::Compilation(format!("LZMA2 compression error: {e}")))?;
    writer.flush()?;

    Ok(())
}

/// Package a `SlideDeck` into a standalone `.slide` file using LZMA2 maximum compression.
pub fn pack_deck_to_file(
    deck: &SlideDeck,
    output_path: &Path,
) -> Result<()> {
    pack_deck_with_assets(deck, None, output_path)
}

/// Helper to compute deterministic temporary cache directory for a package
pub fn get_package_cache_dir(input_path: &Path) -> std::path::PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::new();
    input_path.hash(&mut hasher);
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("deck");
    let hash = hasher.finish();
    std::env::temp_dir()
        .join("cargo_slide_cache")
        .join(format!("{stem}_{hash:016x}"))
}

/// Unpack a `SlideDeck` and its assets from a standalone `.slide` file.
/// Returns the `SlideDeck` and the directory containing extracted assets.
pub fn unpack_deck_and_assets(input_path: &Path) -> Result<(SlideDeck, std::path::PathBuf)> {
    let file = File::open(input_path)?;
    let mut reader = BufReader::new(file);

    // Read initial bytes to check magic
    let mut magic = [0u8; 6];
    let n = reader.read(&mut magic)?;
    let file = File::open(input_path)?;

    let cache_dir = get_package_cache_dir(input_path);
    let _ = std::fs::create_dir_all(&cache_dir);

    // Check if XZ (standard LZMA2 container starts with \xFD7zXZ\x00)
    if n >= 6 && magic == [0xFD, b'7', b'z', b'X', b'Z', 0x00] {
        let decompressor = xz2::read::XzDecoder::new(file);
        let mut tar_archive = tar::Archive::new(decompressor);

        let mut deck_data = None;
        for entry in tar_archive.entries()? {
            let mut entry = entry?;
            let path_str = entry.path()?.to_string_lossy().to_string();
            if path_str == "deck.json" || path_str.ends_with("/deck.json") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                deck_data = Some(content);
            } else if path_str != "manifest.json" && !path_str.ends_with("/manifest.json") {
                let dest = cache_dir.join(&path_str);
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let mut out = File::create(&dest)?;
                std::io::copy(&mut entry, &mut out)?;
            }
        }

        if let Some(data) = deck_data {
            let deck: SlideDeck = serde_json::from_slice(&data)?;
            return Ok((deck, cache_dir));
        }
        return Err(SlideError::Compilation(
            "No deck.json found in .slide archive".to_string(),
        ));
    }

    // Check if ZIP archive (starts with PK\x03\x04)
    if n >= 4 && magic[0..4] == [0x50, 0x4B, 0x03, 0x04] {
        let mut zip_archive = zip::ZipArchive::new(file)?;
        let mut deck_data = None;
        for i in 0..zip_archive.len() {
            let mut entry = zip_archive.by_index(i)?;
            let name = entry.name().to_string();
            if name == "deck.json" || name.ends_with("/deck.json") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                deck_data = Some(content);
            } else if name != "manifest.json"
                && !name.ends_with("/manifest.json")
                && !entry.is_dir()
            {
                let dest = cache_dir.join(&name);
                if let Some(parent) = dest.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let mut out = File::create(&dest)?;
                std::io::copy(&mut entry, &mut out)?;
            }
        }
        if let Some(data) = deck_data {
            let deck: SlideDeck = serde_json::from_slice(&data)?;
            return Ok((deck, cache_dir));
        }
        return Err(SlideError::Compilation(
            "No deck.json found in zip package".to_string(),
        ));
    }

    // Try pure Rust lzma_rs decompressor as fallback
    let mut uncompressed = Vec::new();
    let mut buf_file = BufReader::new(file);
    if lzma_rs::xz_decompress(&mut buf_file, &mut uncompressed).is_ok() || {
        let mut retry_file = BufReader::new(File::open(input_path)?);
        uncompressed.clear();
        lzma_rs::lzma2_decompress(&mut retry_file, &mut uncompressed).is_ok()
    } {
        let mut tar_archive = tar::Archive::new(&uncompressed[..]);
        let mut deck_data = None;
        if let Ok(entries) = tar_archive.entries() {
            for mut entry in entries.flatten() {
                let path_str = entry
                    .path()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if path_str == "deck.json" || path_str.ends_with("/deck.json") {
                    let mut content = Vec::new();
                    entry.read_to_end(&mut content)?;
                    deck_data = Some(content);
                } else if path_str != "manifest.json" && !path_str.ends_with("/manifest.json") {
                    let dest = cache_dir.join(&path_str);
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Ok(mut out) = File::create(&dest) {
                        let _ = std::io::copy(&mut entry, &mut out);
                    }
                }
            }
        }
        if let Some(data) = deck_data {
            let deck: SlideDeck = serde_json::from_slice(&data)?;
            return Ok((deck, cache_dir));
        }
        if let Ok(deck) = serde_json::from_slice::<SlideDeck>(&uncompressed) {
            return Ok((deck, cache_dir));
        }
    }

    Err(SlideError::Compilation(format!(
        "Unsupported or corrupted slide package: {}",
        input_path.display()
    )))
}

/// Unpack a `SlideDeck` directly from raw in-memory bytes and extract assets to cache.
pub fn unpack_deck_and_assets_from_bytes(
    bytes: &[u8],
    package_name: &str,
) -> Result<(SlideDeck, std::path::PathBuf)> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let cache_dir = std::env::temp_dir()
        .join("cargo_slide_cache")
        .join(format!("{package_name}_{hash:016x}"));
    let _ = std::fs::create_dir_all(&cache_dir);

    // Decompress XZ stream
    let cursor = std::io::Cursor::new(bytes);
    let decompressor = xz2::read::XzDecoder::new(cursor);
    let mut tar_archive = tar::Archive::new(decompressor);

    let mut deck_data = None;
    for entry in tar_archive.entries()? {
        let mut entry = entry?;
        let path_str = entry.path()?.to_string_lossy().to_string();
        if path_str == "deck.json" || path_str.ends_with("/deck.json") {
            let mut content = Vec::new();
            entry.read_to_end(&mut content)?;
            deck_data = Some(content);
        } else if path_str != "manifest.json" && !path_str.ends_with("/manifest.json") {
            let dest = cache_dir.join(&path_str);
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut out = File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }

    if let Some(data) = deck_data {
        let deck: SlideDeck = serde_json::from_slice(&data)?;
        return Ok((deck, cache_dir));
    }

    Err(SlideError::Compilation(
        "No deck.json found in embedded slide package".to_string(),
    ))
}

/// Unpack a `SlideDeck` from a standalone `.slide` file.
/// Supports LZMA2 tar archives and legacy/zip archives transparently.
pub fn unpack_deck_from_file(input_path: &Path) -> Result<SlideDeck> {
    unpack_deck_and_assets(input_path).map(|(deck, _)| deck)
}

/// Read only the package metadata without deserializing the whole deck.
pub fn read_package_metadata(input_path: &Path) -> Result<SlidePackageMetadata> {
    let file = File::open(input_path)?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; 6];
    let n = reader.read(&mut magic)?;
    let file = File::open(input_path)?;

    if n >= 6 && magic == [0xFD, b'7', b'z', b'X', b'Z', 0x00] {
        let decompressor = xz2::read::XzDecoder::new(file);
        let mut tar_archive = tar::Archive::new(decompressor);
        for entry in tar_archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_string_lossy().to_string();
            if path == "manifest.json" || path.ends_with("/manifest.json") {
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                let meta: SlidePackageMetadata = serde_json::from_slice(&content)?;
                return Ok(meta);
            }
        }
    }

    // If no manifest found or different format, fallback to full unpack
    let deck = unpack_deck_from_file(input_path)?;
    Ok(SlidePackageMetadata {
        format_version: 1,
        title: deck.title.clone(),
        total_slides: deck.total_slides(),
        default_animation: deck.default_animation,
        compression: "lzma2".to_string(),
        generator: "cargo-slide".to_string(),
    })
}

impl SlideDeck {
    /// Save this presentation deck to a `.slide` package using LZMA2 maximum compression.
    pub fn save_to_package(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<()> {
        pack_deck_to_file(self, path.as_ref())
    }

    /// Load a presentation deck from a `.slide` package.
    pub fn load_from_package(path: impl AsRef<Path>) -> Result<Self> {
        unpack_deck_from_file(path.as_ref())
    }
}
