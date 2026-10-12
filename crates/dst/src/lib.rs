//! Tajima DST compiler and reader.
//!
//! The writer emits a 512-byte header and 3-byte records. Sewing moves that
//! do not fit in ±121 counts are rejected by [`compile_dst`]. [`write_dst`]
//! still splits any remaining oversize delta so a low-level caller cannot
//! emit an illegal record. Jumps are split without adding needle penetrations.
//!
//! Pattern Y grows downward. The file stores the negated Y, which is what
//! pyembroidery does. Read the bytes back with [`read_dst`] or with
//! pyembroidery and the pattern-space coordinates match.

mod compile;
mod read;
mod replay;
mod ternary;

pub use compile::{
    compile_dst, compile_to_vec, compile_to_vec_with_links, write_dst, write_dst_vec,
    write_dst_with, ExportLink, WriteOptions,
};
pub use read::{read_dst, read_dst_with, ReadOptions};
pub use replay::{illustrative_replay, IllustrativeReplay, ReplayEvent};
pub use ternary::{decode_record, encode_record, DecodedRecord, RecordCommand};
