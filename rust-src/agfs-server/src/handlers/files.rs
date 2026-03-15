//! File operation handlers
//!
//! Handles GET/POST/PUT/DELETE requests for /api/v1/files endpoint.

use super::{error_response, map_error_to_status, success_response};
use agfs_sdk::{FileSystem, Streamer, WriteFlag};
use axum::{
    body::Body,
    extract::{Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures::stream::Stream;
use serde::Deserialize;
use std::io::Read;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::mountablefs::MountableFS;

/// Handler state shared across all handlers
#[derive(Clone)]
pub struct HandlerState {
    pub mfs: Arc<MountableFS>,
}

/// Query parameters for file operations
#[derive(Debug, Deserialize)]
pub struct FileQuery {
    /// File path
    pub path: Option<String>,
    /// Read offset (for GET)
    pub offset: Option<i64>,
    /// Read size (for GET)
    pub size: Option<i64>,
    /// Enable streaming (for GET)
    pub stream: Option<bool>,
    /// Recursive delete (for DELETE)
    pub recursive: Option<bool>,
}

/// POST /api/v1/files - Create a new file
pub async fn create_file(
    State(state): State<HandlerState>,
    Query(query): Query<FileQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    state
        .mfs
        .create(&path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response("file created"))
}

/// GET /api/v1/files - Read file content
///
/// Supports both regular reads and streaming reads via the `stream` parameter.
/// When `stream=true`, uses chunked transfer encoding for large files.
pub async fn read_file(
    State(state): State<HandlerState>,
    Query(query): Query<FileQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    let offset = query.offset.unwrap_or(0);
    let size = query.size.unwrap_or(-1);

    // Check if streaming is requested
    if query.stream.unwrap_or(false) {
        // Try to use the Streamer trait if available
        if let Ok(stream_reader) = state.mfs.open_stream(&path) {
            return stream_response(path, stream_reader);
        }

        // Fallback to chunked read using regular file reader
        return stream_read_fallback(&state.mfs, &path, offset, size);
    }

    // Regular non-streaming read
    let data = state
        .mfs
        .read(&path, offset, size)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/octet-stream")],
        data,
    )
        .into_response())
}

/// PUT /api/v1/files - Write file content
pub async fn write_file(
    State(state): State<HandlerState>,
    Query(query): Query<FileQuery>,
    body: Body,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    // Read request body
    let bytes = axum::body::to_bytes(body, 10 * 1024 * 1024) // 10MB limit
        .await
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, "failed to read request body"))?;

    let flags = WriteFlag::CREATE | WriteFlag::TRUNCATE;
    let written = state
        .mfs
        .write(&path, &bytes, -1, flags)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    Ok(success_response(format!("Written {} bytes", written)))
}

/// DELETE /api/v1/files - Delete file or directory
pub async fn delete_file(
    State(state): State<HandlerState>,
    Query(query): Query<FileQuery>,
) -> Result<Response, Response> {
    let path = query.path.ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, "path parameter is required")
    })?;

    let recursive = query.recursive.unwrap_or(false);

    if recursive {
        state
            .mfs
            .remove_all(&path)
            .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;
    } else {
        state
            .mfs
            .remove(&path)
            .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;
    }

    Ok(success_response("deleted"))
}

/// Stream response using StreamReader trait
///
/// This is used when the underlying filesystem supports streaming reads
/// (e.g., streamfs for real-time data).
#[allow(clippy::result_large_err)]
fn stream_response(
    _path: String,
    _stream_reader: Box<dyn agfs_sdk::StreamReader>,
) -> Result<Response, Response> {
    // TODO: Implement full streaming with StreamReader
    // This requires setting up a tokio channel and polling the StreamReader
    // For now, return an error indicating streaming is not fully implemented
    Err(error_response(
        StatusCode::NOT_IMPLEMENTED,
        "StreamReader streaming not yet fully implemented - use regular read",
    ))
}

/// Fallback streaming implementation using regular file reader
///
/// Uses a blocking task to read chunks and sends them through a channel
/// for chunked transfer encoding.
#[allow(clippy::result_large_err)]
fn stream_read_fallback(
    mfs: &MountableFS,
    path: &str,
    offset: i64,
    size: i64,
) -> Result<Response, Response> {
    // Open the file for reading
    let reader = mfs
        .open(path)
        .map_err(|e| error_response(map_error_to_status(&e), e.to_string()))?;

    // Create a channel for sending chunks
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Bytes, axum::Error>>(16);

    // Spawn a blocking task to read chunks
    tokio::task::spawn_blocking(move || {
        let mut reader = reader;
        const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks
        let mut buffer = vec![0u8; CHUNK_SIZE];
        let mut total_read = 0u64;

        // Skip offset bytes
        if offset > 0 {
            let skip_bytes = offset as u64;
            let mut skip_buffer = [0u8; 4096];
            let mut remaining = skip_bytes;
            while remaining > 0 {
                let to_read = std::cmp::min(remaining, skip_buffer.len() as u64) as usize;
                match reader.read(&mut skip_buffer[..to_read]) {
                    Ok(0) => break, // EOF
                    Ok(n) => remaining -= n as u64,
                    Err(_) => break,
                }
            }
        }

        // Read and send chunks
        loop {
            // Check size limit
            let to_read = if size >= 0 {
                let remaining = (size as u64).saturating_sub(total_read);
                if remaining == 0 {
                    break;
                }
                std::cmp::min(CHUNK_SIZE as u64, remaining) as usize
            } else {
                CHUNK_SIZE
            };

            match reader.read(&mut buffer[..to_read]) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    total_read += n as u64;
                    let chunk = Bytes::copy_from_slice(&buffer[..n]);
                    if tx.blocking_send(Ok(chunk)).is_err() {
                        // Receiver dropped
                        break;
                    }
                }
                Err(_) => {
                    let _ = tx.blocking_send(Err(axum::Error::new("read error")));
                    break;
                }
            }
        }
    });

    // Convert the receiver to a stream
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);

    // Create a Body from the stream
    let body = Body::from_stream(stream);

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::TRANSFER_ENCODING, "chunked"),
        ],
        body,
    )
        .into_response())
}

/// Wrapper stream for converting StreamReader to Body
///
/// This allows streaming data from a StreamReader (used by streamfs)
/// through the HTTP response using Server-Sent Events or chunked encoding.
pub struct StreamReaderBody {
    reader: Option<Box<dyn agfs_sdk::StreamReader>>,
    _path: String,
    timeout_ms: u64,
}

impl StreamReaderBody {
    pub fn new(reader: Box<dyn agfs_sdk::StreamReader>, path: String) -> Self {
        Self {
            reader: Some(reader),
            _path: path,
            timeout_ms: 5000, // 5 second default timeout
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

impl Stream for StreamReaderBody {
    type Item = Result<Bytes, axum::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let timeout = self.timeout_ms;
        if let Some(mut reader) = self.reader.take() {
            match reader.read_chunk(timeout) {
                Ok((data, is_eof)) => {
                    if is_eof || data.is_empty() {
                        // Close the reader and end the stream
                        let _ = reader.close();
                        Poll::Ready(None)
                    } else {
                        // Put the reader back for next poll
                        self.reader = Some(reader);
                        Poll::Ready(Some(Ok(Bytes::from(data))))
                    }
                }
                Err(e) => {
                    // Close the reader on error
                    let _ = reader.close();
                    Poll::Ready(Some(Err(axum::Error::new(e.to_string()))))
                }
            }
        } else {
            Poll::Ready(None)
        }
    }
}
