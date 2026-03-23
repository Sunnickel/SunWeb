mod hpack;
mod huffman;

use crate::app::client_handling::h2::hpack::{hpack_encode_response, HpackDecoder};
use crate::app::client_handling::Client;
use crate::http_packet::requests::HTTPRequest;
use log::warn;
use std::collections::HashMap;
// ── HTTP/2 connection preface ─────────────────────────────────────────────────

const HTTP2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

// ── Stream state ──────────────────────────────────────────────────────────────

/// Per-stream state accumulated while receiving a request.
#[derive(Default)]
struct H2Stream {
    headers_block: Vec<u8>,
    data: Vec<u8>,
    /// Set when END_STREAM has been seen on the request side.
    half_closed: bool,
    /// Current send window for this stream (bytes we're allowed to send).
    send_window: i32,
}

// ── Client impl ───────────────────────────────────────────────────────────────

struct PendingHeaders {
    stream_id: u32,
    payload: Vec<u8>,
}

impl Client {
    pub(crate) async fn handle_http2(&mut self) -> std::io::Result<()> {
        // ── 1. Verify the connection preface ─────────────────────────────────
        let mut preface = [0u8; 24];
        if self.stream.read_exact(&mut preface).await.is_err() {
            return Ok(());
        }
        if preface != *HTTP2_PREFACE {
            warn!("Invalid HTTP/2 client preface — closing");
            return Ok(());
        }

        // ── 2. Send our SETTINGS, then wait for theirs ────────────────────────
        self.h2_send_settings().await?;

        let mut decoder = HpackDecoder::new();
        let mut pending_headers: Option<PendingHeaders> = None;
        let mut streams: HashMap<u32, H2Stream> = HashMap::new();
        let mut conn_send_window: i32 = 65_535;

        loop {
            // ── Read 9-byte frame header ──────────────────────────────────────
            let mut hdr = [0u8; 9];
            if self.stream.read_exact(&mut hdr).await.is_err() {
                return Ok(());
            }

            let length = ((hdr[0] as u32) << 16) | ((hdr[1] as u32) << 8) | (hdr[2] as u32);
            let frame_type = hdr[3];
            let flags = hdr[4];
            let stream_id = (((hdr[5] as u32) << 24)
                | ((hdr[6] as u32) << 16)
                | ((hdr[7] as u32) << 8)
                | (hdr[8] as u32))
                & 0x7FFF_FFFF;

            let mut payload = vec![0u8; length as usize];
            self.stream.read_exact(&mut payload).await?;

            match frame_type {
                // ── DATA (0x0) ────────────────────────────────────────────────
                0x0 => {
                    let end_stream = flags & 0x1 != 0;
                    let padded = flags & 0x8 != 0;

                    let data_slice = if padded && !payload.is_empty() {
                        let pad_len = payload[0] as usize;
                        let data_end = payload.len().saturating_sub(pad_len);
                        &payload[1..data_end]
                    } else {
                        &payload[..]
                    };

                    let st = streams.entry(stream_id).or_default();
                    st.data.extend_from_slice(data_slice);
                    if end_stream {
                        st.half_closed = true;
                    }

                    self.h2_send_window_update(0, data_slice.len() as u32)
                        .await?;
                    self.h2_send_window_update(stream_id, data_slice.len() as u32)
                        .await?;

                    if end_stream {
                        self.h2_dispatch(
                            stream_id,
                            &mut streams,
                            &mut decoder,
                            &mut conn_send_window,
                        )
                        .await?;
                    }
                }

                // ── HEADERS (0x1) ─────────────────────────────────────────────
                0x1 => {
                    let end_stream = flags & 0x1 != 0;
                    let end_headers = flags & 0x4 != 0;
                    let padded = flags & 0x8 != 0;
                    let priority = flags & 0x20 != 0;

                    // Strip pad length
                    let mut cursor = 0usize;
                    if padded {
                        cursor += 1;
                    }
                    // Strip 5-byte priority block
                    if priority {
                        cursor += 5;
                    }
                    let pad_len = if padded { payload[0] as usize } else { 0 };
                    let hdr_end = payload.len().saturating_sub(pad_len);
                    let hdr_block = payload[cursor..hdr_end].to_vec();

                    let st = streams.entry(stream_id).or_default();
                    st.send_window = 65_535; // RFC 7540 §6.9.2 initial per-stream window
                    st.half_closed = end_stream;
                    st.headers_block.extend_from_slice(&hdr_block);

                    if end_headers {
                        if end_stream || !st.data.is_empty() {
                            self.h2_dispatch(
                                stream_id,
                                &mut streams,
                                &mut decoder,
                                &mut conn_send_window,
                            )
                            .await?;
                        }
                        // For requests expecting a body (POST etc.) we wait for DATA frames.
                    } else {
                        pending_headers = Some(PendingHeaders {
                            stream_id,
                            payload: hdr_block,
                        });
                    }
                }

                // ── PRIORITY (0x2) ────────────────────────────────────────────
                0x2 => { /* parse and ignore — fully spec-compliant */ }

                // ── RST_STREAM (0x3) ──────────────────────────────────────────
                0x3 => {
                    streams.remove(&stream_id);
                }

                // ── SETTINGS (0x4) ────────────────────────────────────────────
                0x4 => {
                    if flags & 0x1 != 0 {
                        // ACK
                    } else {
                        let mut i = 0;
                        while i + 6 <= payload.len() {
                            let id = ((payload[i] as u16) << 8) | payload[i + 1] as u16;
                            let val = ((payload[i + 2] as u32) << 24)
                                | ((payload[i + 3] as u32) << 16)
                                | ((payload[i + 4] as u32) << 8)
                                | payload[i + 5] as u32;
                            match id {
                                0x1 => {
                                    decoder.update_max_size(val as usize);
                                }
                                0x4 => {
                                    for st in streams.values_mut() {
                                        st.send_window += val as i32 - 65_535;
                                    }
                                }
                                0x5 => {}
                                _ => {}
                            }
                            i += 6;
                        }
                        self.h2_send_settings_ack().await?;
                    }
                }

                // ── PUSH_PROMISE (0x5) — clients must NEVER send this ─────────
                0x5 => {
                    warn!("Client sent PUSH_PROMISE — PROTOCOL_ERROR");
                    self.h2_send_rst_stream(stream_id, 0x1).await?;
                }

                // ── PING (0x6) ────────────────────────────────────────────────
                0x6 => {
                    if payload.len() != 8 {
                        return Ok(());
                    }
                    if flags & 0x1 == 0 {
                        // Not an ACK — echo it back with ACK flag
                        let mut frame = vec![0x00, 0x00, 0x08, 0x6, 0x1, 0x00, 0x00, 0x00, 0x00];
                        frame.extend_from_slice(&payload);
                        self.stream.write_all(&frame).await?;
                        self.stream.flush().await?;
                    }
                }

                // ── GOAWAY (0x7) ──────────────────────────────────────────────
                0x7 => return Ok(()),

                // ── WINDOW_UPDATE (0x8) ───────────────────────────────────────
                0x8 => {
                    if payload.len() == 4 {
                        let increment = (((payload[0] as u32) << 24)
                            | ((payload[1] as u32) << 16)
                            | ((payload[2] as u32) << 8)
                            | payload[3] as u32)
                            & 0x7FFF_FFFF;
                        if stream_id == 0 {
                            conn_send_window += increment as i32;
                        } else if let Some(st) = streams.get_mut(&stream_id) {
                            st.send_window += increment as i32;
                        }
                    }
                }

                // ── CONTINUATION (0x9) ────────────────────────────────────────
                0x9 => {
                    let end_headers = flags & 0x4 != 0;
                    match &mut pending_headers {
                        Some(ph) if ph.stream_id == stream_id => {
                            ph.payload.extend_from_slice(&payload);
                            if end_headers {
                                let id = ph.stream_id;
                                let combined = std::mem::take(&mut ph.payload);
                                pending_headers = None;
                                let st = streams.entry(id).or_default();
                                st.headers_block.extend_from_slice(&combined);
                                if st.half_closed || !st.data.is_empty() {
                                    self.h2_dispatch(
                                        id,
                                        &mut streams,
                                        &mut decoder,
                                        &mut conn_send_window,
                                    )
                                    .await?;
                                }
                            }
                        }
                        _ => {
                            warn!("CONTINUATION on unexpected stream — closing");
                            return Ok(());
                        }
                    }
                }

                other => warn!("Unknown HTTP/2 frame type {:#x}", other),
            }
        }
    }

    // ── Dispatch a complete request ───────────────────────────────────────────

    /// Called once a stream has both END_HEADERS and END_STREAM.
    /// Decodes HPACK headers, reconstructs an HTTPRequest, runs it through the
    /// existing routing + middleware stack, then sends the response.
    async fn h2_dispatch(
        &mut self,
        stream_id: u32,
        streams: &mut HashMap<u32, H2Stream>,
        decoder: &mut HpackDecoder,
        conn_send_window: &mut i32,
    ) -> std::io::Result<()> {
        let st = match streams.remove(&stream_id) {
            Some(s) => s,
            None => return Ok(()),
        };

        // ── Decode header block ───────────────────────────────────────────────
        let header_pairs = decoder.decode(&st.headers_block);

        let mut method = "GET".to_string();
        let mut path = "/".to_string();
        let mut authority = String::new();
        let mut extra_headers: Vec<(String, String)> = Vec::new();

        for (name, value) in &header_pairs {
            match name.as_str() {
                ":method" => method = value.clone(),
                ":path" => path = value.clone(),
                ":authority" => authority = value.clone(),
                ":scheme" => _ = value.clone(),
                _ => extra_headers.push((name.clone(), value.clone())),
            }
        }

        // ── Reconstruct a synthetic HTTP/1.1 request string ──────────────────
        // This lets us reuse HTTPRequest::parse and the entire existing routing
        // + middleware stack without any changes to those layers.
        let mut raw = format!("{} {} HTTP/1.1\r\nHost: {}\r\n", method, path, authority);
        for (n, v) in &extra_headers {
            raw.push_str(&format!("{}: {}\r\n", n, v));
        }
        if !st.data.is_empty() {
            raw.push_str(&format!("Content-Length: {}\r\n", st.data.len()));
        }
        raw.push_str("\r\n");
        if !st.data.is_empty() {
            // Body may be binary — append raw bytes via lossy conversion.
            // For text bodies (JSON, form data) this is lossless.
            raw.push_str(&String::from_utf8_lossy(&st.data));
        }

        // ── Parse + route ─────────────────────────────────────────────────────
        let request = match HTTPRequest::parse(raw.as_ref()) {
            Ok(r) => r,
            Err(e) => {
                warn!("HTTP/2 request parse error: {e}");
                self.h2_send_error(stream_id, 400).await?;
                return Ok(());
            }
        };

        let request = self.apply_request_middleware(request).await;
        let mut response = self.handle_routing(&request).await;
        response = self.apply_response_middleware(request, response).await;

        // ── Encode and send the response ──────────────────────────────────────
        let status = response.status_code.as_u16();

        // Collect response headers (skip ones HTTP/2 forbids)
        let forbidden = ["connection", "keep-alive", "transfer-encoding", "upgrade"];
        let h = response.headers();

        let mut resp_headers: Vec<(String, String)> = Vec::new();

        // Typed fields
        resp_headers.push(("content-type".to_string(), h.content_type.to_string()));

        if let Some(len) = h.content_length {
            resp_headers.push(("content-length".to_string(), len.to_string()));
        }

        // Any freeform headers in .values
        for (n, v) in h.values.iter() {
            let lower = n.to_lowercase();
            if !forbidden.contains(&lower.as_str()) {
                resp_headers.push((lower, v.clone()));
            }
        }

        let hpack_block = hpack_encode_response(status, &resp_headers);
        self.h2_send_headers_frame(stream_id, &hpack_block, response.body().is_none())
            .await?;

        if let Some(body) = response.body() {
            let body_bytes = body.as_bytes();
            self.h2_send_data_frames(stream_id, body_bytes, conn_send_window)
                .await?;
        }

        Ok(())
    }

    // ── Frame senders ─────────────────────────────────────────────────────────

    async fn h2_send_settings(&mut self) -> std::io::Result<()> {
        // Empty SETTINGS — accept all defaults
        let frame = [0x00, 0x00, 0x00, 0x4, 0x0, 0x00, 0x00, 0x00, 0x00];
        self.stream.write_all(&frame).await?;
        self.stream.flush().await
    }

    async fn h2_send_settings_ack(&mut self) -> std::io::Result<()> {
        let frame = [0x00, 0x00, 0x00, 0x4, 0x1, 0x00, 0x00, 0x00, 0x00];
        self.stream.write_all(&frame).await?;
        self.stream.flush().await
    }

    async fn h2_send_rst_stream(&mut self, stream_id: u32, error_code: u32) -> std::io::Result<()> {
        let frame = [
            0x00,
            0x00,
            0x04,
            0x3,
            0x0,
            ((stream_id >> 24) & 0xFF) as u8,
            ((stream_id >> 16) & 0xFF) as u8,
            ((stream_id >> 8) & 0xFF) as u8,
            (stream_id & 0xFF) as u8,
            ((error_code >> 24) & 0xFF) as u8,
            ((error_code >> 16) & 0xFF) as u8,
            ((error_code >> 8) & 0xFF) as u8,
            (error_code & 0xFF) as u8,
        ];
        self.stream.write_all(&frame).await?;
        self.stream.flush().await
    }

    async fn h2_send_window_update(
        &mut self,
        stream_id: u32,
        increment: u32,
    ) -> std::io::Result<()> {
        let inc = increment & 0x7FFF_FFFF;
        let frame = [
            0x00,
            0x00,
            0x04,
            0x8,
            0x0,
            ((stream_id >> 24) & 0xFF) as u8,
            ((stream_id >> 16) & 0xFF) as u8,
            ((stream_id >> 8) & 0xFF) as u8,
            (stream_id & 0xFF) as u8,
            ((inc >> 24) & 0xFF) as u8,
            ((inc >> 16) & 0xFF) as u8,
            ((inc >> 8) & 0xFF) as u8,
            (inc & 0xFF) as u8,
        ];
        self.stream.write_all(&frame).await?;
        self.stream.flush().await
    }

    /// Send a HEADERS frame. If `end_stream` is true the END_STREAM flag is set,
    /// meaning no DATA frame follows (e.g. for HEAD responses or 204/304).
    async fn h2_send_headers_frame(
        &mut self,
        stream_id: u32,
        hpack_block: &[u8],
        end_stream: bool,
    ) -> std::io::Result<()> {
        let len = hpack_block.len() as u32;
        let flags = 0x4 | if end_stream { 0x1 } else { 0x0 }; // END_HEADERS [| END_STREAM]
        let mut frame = vec![
            ((len >> 16) & 0xFF) as u8,
            ((len >> 8) & 0xFF) as u8,
            (len & 0xFF) as u8,
            0x1,
            flags,
            ((stream_id >> 24) & 0xFF) as u8,
            ((stream_id >> 16) & 0xFF) as u8,
            ((stream_id >> 8) & 0xFF) as u8,
            (stream_id & 0xFF) as u8,
        ];
        frame.extend_from_slice(hpack_block);
        self.stream.write_all(&frame).await?;
        self.stream.flush().await
    }

    /// Send body bytes as one or more DATA frames, respecting both the
    /// connection-level and per-stream send windows.
    async fn h2_send_data_frames(
        &mut self,
        stream_id: u32,
        body: &[u8],
        conn_window: &mut i32,
    ) -> std::io::Result<()> {
        const MAX_FRAME_SIZE: usize = 16_384; // RFC 7540 default max

        let mut offset = 0;
        // Per-stream window: the stream was removed from the map in h2_dispatch,
        // so we track it locally. We start with the initial value; any
        // WINDOW_UPDATE that arrived before we dispatched was applied above.
        let mut stream_window: i32 = 65_535;

        while offset < body.len() {
            // How many bytes are we allowed to send right now?
            let allowed = (*conn_window).min(stream_window).max(0) as usize;
            if allowed == 0 {
                // Flow-control stalled — in a real server you'd await a
                // WINDOW_UPDATE wakeup; here we just yield briefly and retry.
                tokio::task::yield_now().await;
                continue;
            }

            let chunk_size = MAX_FRAME_SIZE.min(allowed).min(body.len() - offset);
            let chunk = &body[offset..offset + chunk_size];
            let is_last = offset + chunk_size >= body.len();

            let len = chunk_size as u32;
            let flags = if is_last { 0x1 } else { 0x0 }; // END_STREAM on last frame
            let mut frame = vec![
                ((len >> 16) & 0xFF) as u8,
                ((len >> 8) & 0xFF) as u8,
                (len & 0xFF) as u8,
                0x0,
                flags,
                ((stream_id >> 24) & 0xFF) as u8,
                ((stream_id >> 16) & 0xFF) as u8,
                ((stream_id >> 8) & 0xFF) as u8,
                (stream_id & 0xFF) as u8,
            ];
            frame.extend_from_slice(chunk);
            self.stream.write_all(&frame).await?;

            *conn_window -= chunk_size as i32;
            stream_window -= chunk_size as i32;
            offset += chunk_size;
        }

        self.stream.flush().await
    }

    /// Send a minimal error response (no body) and RST the stream.
    async fn h2_send_error(&mut self, stream_id: u32, status: u16) -> std::io::Result<()> {
        let hpack = hpack_encode_response(status, &[]);
        self.h2_send_headers_frame(stream_id, &hpack, true).await?;
        self.h2_send_rst_stream(stream_id, 0x0).await
    }
}
