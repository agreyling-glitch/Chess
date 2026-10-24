//! Bounded native ZIP reader. Archive paths are never extracted onto the filesystem.
use std::io::Read;
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
fn word(bytes: &[u8], at: usize) -> Result<usize, String> {
    let data = bytes.get(at..at + 2).ok_or("Damaged ZIP directory")?;
    Ok(u16::from_le_bytes(data.try_into().unwrap()) as usize)
}
fn number(bytes: &[u8], at: usize) -> Result<usize, String> {
    let data = bytes.get(at..at + 4).ok_or("Damaged ZIP directory")?;
    Ok(u32::from_le_bytes(data.try_into().unwrap()) as usize)
}
fn crc(bytes: &[u8]) -> u32 {
    let table: [u32; 256] = std::array::from_fn(|n| {
        let mut v = n as u32;
        for _ in 0..8 {
            v = if v & 1 != 0 {
                0xedb88320 ^ (v >> 1)
            } else {
                v >> 1
            };
        }
        v
    });
    bytes.iter().fold(0xffffffff, |v, b| {
        table[((v ^ u32::from(*b)) & 255) as usize] ^ (v >> 8)
    }) ^ 0xffffffff
}
pub fn pgns(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    if bytes.len() > MAX_BYTES {
        return Err("ZIP exceeds 64 MiB".into());
    }
    let end = (bytes.len().saturating_sub(65557)..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            number(bytes, i) == Ok(0x06054b50)
                && word(bytes, i + 20).is_ok_and(|size| i + 22 + size == bytes.len())
        })
        .ok_or("Damaged ZIP file")?;
    let count = word(bytes, end + 10)?;
    let size = number(bytes, end + 12)?;
    let start = number(bytes, end + 16)?;
    if word(bytes, end + 4)? != 0
        || word(bytes, end + 6)? != 0
        || word(bytes, end + 8)? != count
        || count == 65535
        || size == 0xffffffff
        || start == 0xffffffff
    {
        return Err("Multi-part and ZIP64 archives are not supported".into());
    }
    if count > 1000 {
        return Err("ZIP exceeds 1,000 entries".into());
    }
    if start.checked_add(size) != Some(end) {
        return Err("Damaged ZIP directory".into());
    }
    let mut offset = start;
    let mut total = 0;
    let mut files = Vec::new();
    for _ in 0..count {
        if offset + 46 > end || number(bytes, offset)? != 0x02014b50 {
            return Err("Damaged ZIP entry".into());
        }
        let flags = word(bytes, offset + 8)?;
        let method = word(bytes, offset + 10)?;
        let expected = number(bytes, offset + 16)? as u32;
        let compressed = number(bytes, offset + 20)?;
        let expanded = number(bytes, offset + 24)?;
        let names = word(bytes, offset + 28)?;
        let extra = word(bytes, offset + 30)?;
        let comment = word(bytes, offset + 32)?;
        let disk = word(bytes, offset + 34)?;
        let local = number(bytes, offset + 42)?;
        let next = offset + 46 + names + extra + comment;
        if next > end {
            return Err("Damaged ZIP filename".into());
        }
        let raw = &bytes[offset + 46..offset + 46 + names];
        let name = String::from_utf8_lossy(raw).into_owned();
        if name.to_ascii_lowercase().ends_with(".pgn")
            && !name.starts_with("__MACOSX/")
            && !name.split('/').any(|part| part.starts_with("._"))
        {
            if flags & 0x41 != 0 {
                return Err("Password-protected ZIP files must be unzipped first".into());
            }
            if ![0, 8].contains(&method)
                || disk != 0
                || [compressed, expanded, local].contains(&0xffffffff)
            {
                return Err("Unsupported ZIP entry".into());
            }
            total += expanded;
            if total > MAX_BYTES {
                return Err("Uncompressed PGNs exceed 64 MiB".into());
            }
            if local.checked_add(30).is_none_or(|end| end > start)
                || number(bytes, local)? != 0x04034b50
                || word(bytes, local + 6)? != flags
                || word(bytes, local + 8)? != method
            {
                return Err("Damaged ZIP local header".into());
            }
            let local_names = word(bytes, local + 26)?;
            let data_start = local + 30 + local_names + word(bytes, local + 28)?;
            if data_start
                .checked_add(compressed)
                .is_none_or(|end| end > start)
                || local_names != raw.len()
                || bytes.get(local + 30..local + 30 + local_names) != Some(raw)
            {
                return Err("Damaged ZIP data".into());
            }
            let data = &bytes[data_start..data_start + compressed];
            let decoded = if method == 0 {
                data.to_vec()
            } else {
                let mut output = Vec::new();
                flate2::read::DeflateDecoder::new(data)
                    .take(expanded.min(MAX_BYTES) as u64 + 1)
                    .read_to_end(&mut output)
                    .map_err(|_| "Corrupt ZIP compression")?;
                output
            };
            if decoded.len() != expanded || crc(&decoded) != expected {
                return Err("ZIP integrity check failed".into());
            }
            files.push((name, decoded));
        }
        offset = next;
    }
    if offset != end || files.is_empty() {
        return Err("ZIP contains no PGN files or has an invalid directory".into());
    }
    Ok(files)
}

#[cfg(test)]
pub(crate) fn test_crc(bytes: &[u8]) -> u32 {
    crc(bytes)
}
