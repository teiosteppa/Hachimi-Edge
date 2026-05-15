use fnv::FnvHashMap;
use once_cell::sync::OnceCell;
use std::ffi::CString;
use std::sync::Mutex;

static EXPORT_MAP: OnceCell<FnvHashMap<String, usize>> = OnceCell::new();
static PATCHES: Mutex<Vec<(&'static str, usize)>> = Mutex::new(Vec::new());

pub fn init_exports(header_addr: usize, slide: isize) {
    info!("iOS: Parsing Mach-O Export Trie manually from live memory...");
    match unsafe { parse_macho_exports(header_addr, slide) } {
        Ok(map) => {
            let count = map.len();
            if EXPORT_MAP.set(map).is_ok() {
                info!("iOS: Successfully mapped {} IL2CPP exports! (Order-independent)", count);
            }
        }
        Err(e) => error!("iOS: Export trie parsing failed: {}", e),
    }
}

unsafe fn parse_macho_exports(header_addr: usize, slide: isize) -> Result<FnvHashMap<String, usize>, &'static str> {
    let header = header_addr as *const u8;

    let magic = u32::from_le_bytes(*(header as *const [u8; 4]));
    if magic != 0xfeedfacf && magic != 0xfeedface {
        return Err("Invalid Mach-O magic header");
    }

    let ncmds = u32::from_le_bytes(*(header.add(16) as *const [u8; 4]));
    let sizeofcmds = u32::from_le_bytes(*(header.add(20) as *const [u8; 4])) as usize;

    let mut offset = 32usize;
    let max_offset = 32 + sizeofcmds;

    let mut export_off = 0u32;
    let mut export_size = 0u32;
    let mut linkedit_vmaddr = 0u64;
    let mut linkedit_fileoff = 0u64;

    for _ in 0..ncmds {
        if offset + 8 > max_offset { break; }

        let cmd = u32::from_le_bytes(*(header.add(offset) as *const [u8; 4]));
        let cmdsize = u32::from_le_bytes(*(header.add(offset + 4) as *const [u8; 4])) as usize;

        if cmdsize == 0 { break; }

        if cmd == 0x22 || cmd == 0x80000022 {
            info!("iOS: Found LC_DYLD_INFO_ONLY at offset {}", offset);
            export_off = u32::from_le_bytes(*(header.add(offset + 40) as *const [u8; 4]));
            export_size = u32::from_le_bytes(*(header.add(offset + 44) as *const [u8; 4]));
        } else if cmd == 0x33 || cmd == 0x80000033 {
            info!("iOS: Found LC_DYLD_EXPORTS_TRIE at offset {}", offset);
            export_off = u32::from_le_bytes(*(header.add(offset + 8) as *const [u8; 4]));
            export_size = u32::from_le_bytes(*(header.add(offset + 12) as *const [u8; 4]));
        } else if cmd == 0x19 {
            let segname = std::slice::from_raw_parts(header.add(offset + 8), 16);
            if segname.starts_with(b"__LINKEDIT") {
                linkedit_vmaddr = u64::from_le_bytes(*(header.add(offset + 24) as *const [u8; 8]));
                linkedit_fileoff = u64::from_le_bytes(*(header.add(offset + 40) as *const [u8; 8]));
            }
        }
        offset += cmdsize;
    }

    if export_off == 0 || export_size == 0 { return Err("No export trie found in Mach-O header"); }
    if linkedit_fileoff == 0 { return Err("__LINKEDIT segment not found"); }

    let trie_va = (linkedit_vmaddr as i64 + slide as i64) as u64 + (export_off as u64 - linkedit_fileoff);
    let trie_data = std::slice::from_raw_parts(trie_va as *const u8, export_size as usize);

    let mut map = FnvHashMap::default();
    let mut stack = vec![(0usize, String::new())];

    while let Some((node_off, prefix)) = stack.pop() {
        if node_off >= trie_data.len() { continue; }
        let mut i = node_off;

        let (terminal_size, len) = decode_uleb128(&trie_data[i..]);
        if len == 0 { continue; }
        i += len;

        if terminal_size > 0 {
            if i >= trie_data.len() { continue; }
            let (flags, len_flags) = decode_uleb128(&trie_data[i..]);
            if len_flags == 0 { continue; }
            let addr_offset = i + len_flags;

            if (flags & 0x08) == 0 && (flags & 0x10) == 0 {
                if addr_offset < trie_data.len() {
                    let (symbol_offset, _) = decode_uleb128(&trie_data[addr_offset..]);
                    let actual_addr = header_addr + symbol_offset as usize;

                    let clean_name = if prefix.starts_with('_') { &prefix[1..] } else { &prefix[..] };
                    if clean_name.starts_with("il2cpp_") {
                        map.insert(clean_name.to_string(), actual_addr);
                    }
                }
            }
            i += terminal_size as usize;
        }

        if i >= trie_data.len() { continue; }
        let child_count = trie_data[i];
        i += 1;

        for _ in 0..child_count {
            let mut child_str = prefix.clone();
            while i < trie_data.len() && trie_data[i] != 0 {
                child_str.push(trie_data[i] as char);
                i += 1;
            }
            i += 1;

            if i >= trie_data.len() { break; }
            let (child_node_off, len_offset) = decode_uleb128(&trie_data[i..]);
            if len_offset == 0 { break; }
            i += len_offset;

            stack.push((child_node_off as usize, child_str));
        }
    }

    Ok(map)
}

fn decode_uleb128(data: &[u8]) -> (u64, usize) {
    if data.is_empty() { return (0, 0); }
    let mut result: u64 = 0;
    let mut shift = 0;
    for (i, &b) in data.iter().enumerate() {
        result |= ((b & 0x7F) as u64) << shift;
        shift += 7;
        if b & 0x80 == 0 { return (result, i + 1); }
        if shift >= 64 { break; }
    }
    (0, 0)
}

pub unsafe fn dlsym(handle: *mut std::os::raw::c_void, name: &str) -> usize {
    if let Ok(patches) = PATCHES.lock() {
        if let Some(&(_, addr)) = patches.iter().find(|(k, _)| *k == name) {
            if addr != 0 { return addr; }
        }
    }

    if let Some(map) = EXPORT_MAP.get() {
        if let Some(&addr) = map.get(name) {
            return addr;
        }
    }

    if !handle.is_null() {
        if let Ok(c_name) = CString::new(name) {
            let addr = libc::dlsym(handle, c_name.as_ptr());
            if !addr.is_null() { return addr as usize; }

            let underscored = format!("_{}", name);
            if let Ok(c_under) = CString::new(underscored) {
                let addr_under = libc::dlsym(handle, c_under.as_ptr());
                if !addr_under.is_null() { return addr_under as usize; }
            }
        }
    }

    warn!("iOS: symbol not resolved: {}", name);
    0
}