use crate::app::client_handling::h2::huffman::{
    hpack_literal_huffman, hpack_string_huffman, huffman_decode,
};
// ── HPACK static table (RFC 7541 Appendix A, 61 entries) ─────────────────────
pub(super) const HPACK_STATIC_TABLE: &[(&str, &str)] = &[
    ("", ""),                             //  0  (1-based, slot 0 unused)
    (":authority", ""),                   //  1
    (":method", "GET"),                   //  2
    (":method", "POST"),                  //  3
    (":path", "/"),                       //  4
    (":path", "/index.html"),             //  5
    (":scheme", "http"),                  //  6
    (":scheme", "https"),                 //  7
    (":status", "200"),                   //  8
    (":status", "204"),                   //  9
    (":status", "206"),                   // 10
    (":status", "304"),                   // 11
    (":status", "400"),                   // 12
    (":status", "404"),                   // 13
    (":status", "500"),                   // 14
    ("accept-charset", ""),               // 15
    ("accept-encoding", "gzip, deflate"), // 16
    ("accept-language", ""),              // 17
    ("accept-ranges", ""),                // 18
    ("accept", ""),                       // 19
    ("access-control-allow-origin", ""),  // 20
    ("age", ""),                          // 21
    ("allow", ""),                        // 22
    ("authorization", ""),                // 23
    ("cache-control", ""),                // 24
    ("content-disposition", ""),          // 25
    ("content-encoding", ""),             // 26
    ("content-language", ""),             // 27
    ("content-length", ""),               // 28
    ("content-location", ""),             // 29
    ("content-range", ""),                // 30
    ("content-type", ""),                 // 31
    ("cookie", ""),                       // 32
    ("date", ""),                         // 33
    ("etag", ""),                         // 34
    ("expect", ""),                       // 35
    ("expires", ""),                      // 36
    ("from", ""),                         // 37
    ("host", ""),                         // 38
    ("if-match", ""),                     // 39
    ("if-modified-since", ""),            // 40
    ("if-none-match", ""),                // 41
    ("if-range", ""),                     // 42
    ("if-unmodified-since", ""),          // 43
    ("last-modified", ""),                // 44
    ("link", ""),                         // 45
    ("location", ""),                     // 46
    ("max-forwards", ""),                 // 47
    ("proxy-authenticate", ""),           // 48
    ("proxy-authorization", ""),          // 49
    ("range", ""),                        // 50
    ("referer", ""),                      // 51
    ("refresh", ""),                      // 52
    ("retry-after", ""),                  // 53
    ("server", ""),                       // 54
    ("set-cookie", ""),                   // 55
    ("strict-transport-security", ""),    // 56
    ("transfer-encoding", ""),            // 57
    ("user-agent", ""),                   // 58
    ("vary", ""),                         // 59
    ("via", ""),                          // 60
    ("www-authenticate", ""),             // 61
];

// ── HPACK decoder ─────────────────────────────────────────────────────────────

/// Minimal HPACK decoder (RFC 7541).
/// Supports indexed, literal-with-indexing, literal-without-indexing, and
/// literal-never-indexed representations. Dynamic table eviction included.
pub(super) struct HpackDecoder {
    dynamic: Vec<(String, String)>, // newest first
    max_size: usize,
    current_size: usize,
}

impl HpackDecoder {
    pub(super) fn new() -> Self {
        HpackDecoder {
            dynamic: Vec::new(),
            max_size: 4096,
            current_size: 0,
        }
    }

    /// Look up an index (1-based) across static then dynamic table.
    pub(super) fn get(&self, idx: usize) -> Option<(String, String)> {
        if idx == 0 {
            return None;
        }
        if idx < HPACK_STATIC_TABLE.len() {
            let (n, v) = HPACK_STATIC_TABLE[idx];
            return Some((n.to_string(), v.to_string()));
        }
        let dyn_idx = idx - HPACK_STATIC_TABLE.len() + 1; // 1-based into dynamic
        self.dynamic.get(dyn_idx - 1).cloned()
    }

    pub(super) fn add_to_dynamic(&mut self, name: String, value: String) {
        let entry_size = name.len() + value.len() + 32;
        self.current_size += entry_size;
        self.dynamic.insert(0, (name, value));
        while self.current_size > self.max_size {
            if let Some(evicted) = self.dynamic.pop() {
                self.current_size = self
                    .current_size
                    .saturating_sub(evicted.0.len() + evicted.1.len() + 32);
            } else {
                break;
            }
        }
    }

    pub(super) fn update_max_size(&mut self, new_max: usize) {
        self.max_size = new_max;
        while self.current_size > self.max_size {
            if let Some(evicted) = self.dynamic.pop() {
                self.current_size = self
                    .current_size
                    .saturating_sub(evicted.0.len() + evicted.1.len() + 32);
            } else {
                break;
            }
        }
    }

    /// Decode a full HPACK header block into a list of (name, value) pairs.
    pub(super) fn decode(&mut self, data: &[u8]) -> Vec<(String, String)> {
        let mut headers = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            let b = data[pos];

            if b & 0x80 != 0 {
                // §6.1 Indexed header field
                let (idx, adv) = hpack_decode_int(&data[pos..], 7);
                pos += adv;
                if let Some(pair) = self.get(idx) {
                    headers.push(pair);
                }
            } else if b & 0x40 != 0 {
                // §6.2.1 Literal with incremental indexing
                let (idx, adv) = hpack_decode_int(&data[pos..], 6);
                pos += adv;
                let (name, adv2) = if idx == 0 {
                    hpack_decode_string(&data[pos..])
                } else {
                    let n = self.get(idx).map(|(n, _)| n).unwrap_or_default();
                    (n, 0)
                };
                pos += adv2;
                let (value, adv3) = hpack_decode_string(&data[pos..]);
                pos += adv3;
                self.add_to_dynamic(name.clone(), value.clone());
                headers.push((name, value));
            } else if b & 0x20 != 0 {
                // §6.3 Dynamic table size update
                let (new_size, adv) = hpack_decode_int(&data[pos..], 5);
                pos += adv;
                self.update_max_size(new_size);
            } else {
                // §6.2.2 / §6.2.3 Literal without / never indexing
                let (idx, adv) = hpack_decode_int(&data[pos..], 4);
                pos += adv;
                let (name, adv2) = if idx == 0 {
                    hpack_decode_string(&data[pos..])
                } else {
                    let n = self.get(idx).map(|(n, _)| n).unwrap_or_default();
                    (n, 0)
                };
                pos += adv2;
                let (value, adv3) = hpack_decode_string(&data[pos..]);
                pos += adv3;
                headers.push((name, value));
            }
        }

        headers
    }
}

/// Decode an HPACK integer (RFC 7541 §5.1).
/// Returns (value, bytes_consumed).
pub(super) fn hpack_decode_int(data: &[u8], prefix_bits: u8) -> (usize, usize) {
    let mask = (1u8 << prefix_bits) - 1;
    let first = (data[0] & mask) as usize;
    if first < mask as usize {
        return (first, 1);
    }
    let mut value = first;
    let mut m = 0usize;
    let mut i = 1;
    while i < data.len() {
        let b = data[i];
        value += ((b & 0x7F) as usize) << m;
        m += 7;
        i += 1;
        if b & 0x80 == 0 {
            break;
        }
    }
    (value, i)
}

/// Decode an HPACK string (RFC 7541 §5.2), Huffman or plain.
/// Returns (string, bytes_consumed).
pub(super) fn hpack_decode_string(data: &[u8]) -> (String, usize) {
    if data.is_empty() {
        return (String::new(), 0);
    }
    let huffman = data[0] & 0x80 != 0;
    let (len, adv) = hpack_decode_int(data, 7);
    let bytes = &data[adv..adv + len];
    let result = if huffman {
        huffman_decode(bytes)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    (result, adv + len)
}

/// Encode response headers into an HPACK block.
/// Uses indexed representation for :status when it hits a static entry,
/// literal-without-indexing + Huffman for everything else.
pub(super) fn hpack_encode_response(status: u16, headers: &[(String, String)]) -> Vec<u8> {
    let mut buf = Vec::new();

    // :status — check static table entries 8-14
    let status_str = status.to_string();
    let static_status_idx: Option<usize> = HPACK_STATIC_TABLE
        .iter()
        .enumerate()
        .skip(8)
        .find(|(_, (n, v))| *n == ":status" && *v == status_str.as_str())
        .map(|(i, _)| i);

    if let Some(idx) = static_status_idx {
        buf.push(0x80 | idx as u8); // fully indexed
    } else {
        // Literal without indexing, static name index 8 (:status), Huffman value
        buf.push(0x08); // index 8, literal without indexing
        buf.extend(hpack_string_huffman(status_str.as_bytes()));
    }

    // Remaining headers — literal without indexing, Huffman name + value
    for (name, value) in headers {
        buf.extend(hpack_literal_huffman(name.as_bytes(), value.as_bytes()));
    }

    buf
}
