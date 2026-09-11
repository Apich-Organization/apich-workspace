mod common;
use apich_vcs::ContentAddressableStorage;
use apich_vcs::FastCdc;
use apich_vcs::FastCdcConfig;
use common::test_temp_dir;

#[test]
fn test_fastcdc_chunk_bounds() {
    let config = FastCdcConfig {
        min_size: 1024,
        avg_size: 4096,
        max_size: 16384,
    };

    // Small data (< min_size)
    let small_data = vec![42u8; 500];
    let chunks: Vec<_> = FastCdc::new(&small_data, config).collect();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].length, 500);

    // Large data (100 KB)
    let mut large_data = Vec::with_capacity(100 * 1024);
    for i in 0..(100 * 1024) {
        large_data.push((i % 251) as u8);
    }

    let chunks: Vec<_> = FastCdc::new(&large_data, config).collect();
    assert!(chunks.len() > 5);

    let mut total_len = 0;
    for chunk in &chunks {
        total_len += chunk.length;
        assert!(chunk.length <= config.max_size);
        // All but the very last chunk should be at least min_size
        if chunk.offset + chunk.length < large_data.len() {
            assert!(chunk.length >= config.min_size);
        }
    }
    assert_eq!(total_len, large_data.len());
}

#[test]
fn test_fastcdc_boundary_stability_and_deduplication() {
    let config = FastCdcConfig {
        min_size: 1024,
        avg_size: 4096,
        max_size: 16384,
    };

    // Generate 64 KB of text paragraphs
    let paragraph = "In technical and academic writing, version control must accommodate large binary datasets, \
        vector graphics, SQLite cache files, and rapid document drafts without repository bloat or staging friction.\n";
    let base_data = paragraph.repeat(400).into_bytes();

    let base_chunks: Vec<_> = FastCdc::new(&base_data, config).collect();
    assert!(base_chunks.len() >= 4);

    // Prepend 100 bytes of title and author header
    let mut modified_data =
        b"# Thesis Draft v2: Modern Deduplication in Technical VCS\nBy Alice Researcher\n\n"
            .to_vec();
    modified_data.extend_from_slice(&base_data);

    let modified_chunks: Vec<_> = FastCdc::new(&modified_data, config).collect();

    // Content-Defined Chunking: after the modified prefix, chunk boundaries resynchronize!
    let mut deduplicated_count = 0;
    for b in &base_chunks {
        if modified_chunks.iter().any(|m| m.data == b.data) {
            deduplicated_count += 1;
        }
    }
    assert!(
        deduplicated_count >= base_chunks.len() - 2,
        "Majority of chunks should be shared: {} / {}",
        deduplicated_count,
        base_chunks.len()
    );
}

#[test]
fn test_cas_store_and_reconstruct() {
    let temp = test_temp_dir();
    let cas = ContentAddressableStorage::new(temp.path()).unwrap();

    let config = FastCdcConfig::document();
    let original_text =
        "This is an academic research paper on Quantum Superposition.\n".repeat(300);

    let (chunks, blake3_hash, git_sha1) =
        cas.put_file_data(original_text.as_bytes(), config).unwrap();

    assert!(!chunks.is_empty());
    assert!(!blake3_hash.is_empty());
    assert!(!git_sha1.is_empty());

    // Reconstruct
    let reconstructed_bytes = cas.read_file_data(&chunks).unwrap();
    let reconstructed_text = String::from_utf8(reconstructed_bytes).unwrap();

    assert_eq!(reconstructed_text, original_text);
}
