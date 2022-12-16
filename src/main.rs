/*
BSD 3-Clause License

Copyright (c) 2022, Stephen Bates
Copyright (c) 2022, Guillermo Cifuentes, gcifuentes@escandasoft.eu
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

use clap::command;
use clap::Parser;
use metrics::increment_counter;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::{
    error::Error,
    fs::File,
    io::{self, Read},
    mem::size_of,
    net::SocketAddr,
    os::unix::prelude::{AsRawFd, FromRawFd},
    process::{Command, Stdio},
    str::{self, FromStr},
};

mod blktrace {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_fmt_panics)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    pub fn blk_tc_act(act: u32) -> u32 {
        act << BLK_TC_SHIFT
    }
}

static STDIN_PATH: &str = "-";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// subcommand
    #[command(subcommand)]
    command: Subcommand,
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    Fork {
        /// list of devices to blktrace
        #[arg(short, long)]
        device: Vec<String>,
    },
    Ingest {
        /// Input file path, - for stdin
        #[arg(short, long, default_value_t = STDIN_PATH.to_string(),)]
        input: String,
    },
}

const FRAGMENT_SIZE: usize = size_of::<blktrace::blk_io_trace>();

async fn process_input(input: &mut dyn Read) -> Result<(), Box<dyn Error>> {
    let mut buffer: [u8; FRAGMENT_SIZE] = [0; FRAGMENT_SIZE];
    while let Ok(()) = input.read_exact(&mut buffer) {
        let trace: blktrace::blk_io_trace = unsafe { std::mem::transmute(buffer) };
        let mut str_vec = Vec::<u8>::with_capacity(trace.pdu_len.into());
        io::copy(&mut input.take(trace.pdu_len.into()), &mut str_vec)?;
        //let str: String = String::from(str::f+rom_utf8(&str_vec)?);
        //println!("str: {}", str);
        increment_counter!("iowatcherng-exporter.packets_read");
        if (trace.magic & 0xffffff00) != blktrace::BLK_IO_TRACE_MAGIC {
            println!("Bad pkt magic");
        } else {
            if (trace.action & !blktrace::blktrace_notify___BLK_TN_CGROUP) == blktrace::blktrace_notify___BLK_TN_MESSAGE
            {
                println!("NOTIFY");
                match trace.action & !blktrace::blktrace_notify___BLK_TN_CGROUP {
                    blktrace::blktrace_notify___BLK_TN_PROCESS => println!("PROCESS"),
                    blktrace::blktrace_notify___BLK_TN_TIMESTAMP => println!("TS"),
                    blktrace::blktrace_notify___BLK_TN_MESSAGE => println!("MS"),
                    _ => println!("Unk NOTIFY"),
                }
            } else if (trace.action & blktrace::blk_tc_act(blktrace::blktrace_cat_BLK_TC_PC)) == 0 {
                println!("PC");
                let _w = (trace.action & blktrace::blk_tc_act(blktrace::blktrace_cat_BLK_TC_WRITE)) != 0;
                let act = (trace.action & 0xffff) & !blktrace::blktrace_act___BLK_TA_CGROUP;
                match act {
                    blktrace::blktrace_act___BLK_TA_QUEUE => println!("TQ"),
                    blktrace::blktrace_act___BLK_TA_GETRQ => println!("RQ"),
                    blktrace::blktrace_act___BLK_TA_SLEEPRQ => println!("SPRQ"),
                    blktrace::blktrace_act___BLK_TA_REQUEUE => println!("REQ"),
                    blktrace::blktrace_act___BLK_TA_ISSUE => println!("ISSUE"),
                    blktrace::blktrace_act___BLK_TA_COMPLETE => println!("COMPLETE"),
                    blktrace::blktrace_act___BLK_TA_INSERT => println!("INSERT"),
                    _ => println!("Unk PC"),
                }
            } else {
                println!("CGROUP");
                let _w = (trace.action & blktrace::blk_tc_act(blktrace::blktrace_cat_BLK_TC_WRITE)) != 0;
                let act = (trace.action & 0xffff) & !blktrace::blktrace_act___BLK_TA_CGROUP;
                match act {
                    blktrace::blktrace_act___BLK_TA_QUEUE => println!("TQ"),
                    blktrace::blktrace_act___BLK_TA_INSERT => println!("RQ"),
                    blktrace::blktrace_act___BLK_TA_BACKMERGE => println!("SPRQ"),
                    blktrace::blktrace_act___BLK_TA_FRONTMERGE => println!("REQ"),
                    blktrace::blktrace_act___BLK_TA_GETRQ => println!("ISSUE"),
                    blktrace::blktrace_act___BLK_TA_SLEEPRQ => println!("COMPLETE"),
                    blktrace::blktrace_act___BLK_TA_REQUEUE => println!("INSERT"),
                    _ => println!("Unk CGROUP"),
                }
            }
        }
        buffer = [0; FRAGMENT_SIZE];
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    PrometheusBuilder::new()
        .with_http_listener(SocketAddr::from_str("::9975").unwrap())
        .install()
        .expect("failed to install Prometheus recoder/exporter");

    match &args.command {
        Subcommand::Fork { device } => {
            let mut arg_stack = Vec::new();
            for dev in device {
                arg_stack.push("-d".to_string());
                arg_stack.push(dev.to_string());
            }
            arg_stack.push("-o".to_string());
            arg_stack.push("-".to_string());
            match Command::new("blktrace")
                .args(arg_stack)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
            {
                Err(why) => panic!("couldn't spawn blktrace: {}", why),
                Ok(mut child) => {
                    let mut stdout = child.stdout.expect("stdout is opened at this time");
                    process_input(&mut stdout).await?;
                },
            };
        },
        Subcommand::Ingest { input } => {
            let mut input = if input.eq(&STDIN_PATH) {
                unsafe { File::from_raw_fd(io::stdin().as_raw_fd()) }
            } else {
                File::open(input)?
            };
            process_input(&mut input).await?;
        },
    };
    Ok(())
}
