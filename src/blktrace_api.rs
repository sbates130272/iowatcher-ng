#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_fmt_panics)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::mem::size_of;

pub const FRAGMENT_SIZE: usize = size_of::<blk_io_trace>();

pub fn blk_tc_act(act: u32) -> u32 {
    act << BLK_TC_SHIFT
}
