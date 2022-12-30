#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_fmt_panics)]

use std::mem::size_of;
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub const FRAGMENT_SIZE: usize = size_of::<blk_io_trace>();

pub fn blk_tc_act(act: u32) -> u32 {
    act << BLK_TC_SHIFT
}
