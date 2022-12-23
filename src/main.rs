/*
BSD 3-Clause License

Copyright (c) 2022, Stephen Bates <sbates@raithlin.com>
Copyright (c) 2022, Guillermo Cifuentes <gcifuentes@escandasoft.eu>
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

use bytes::Bytes;
use clap::command;
use clap::Parser;
use crossbeam::deque::Stealer;
use crossbeam::deque::Worker;
use metrics::*;
use metrics_exporter_prometheus::PrometheusBuilder;
use quinn::Endpoint;
use quinn::ServerConfig;
use std::borrow::BorrowMut;
use std::fs::read;
use std::fs::File;
use std::io::BufReader;
use std::iter::FusedIterator;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use std::{
    error::Error,
    io::{self, Read},
    mem::size_of,
    net::SocketAddr,
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
    command: CmdKind,
}

#[derive(clap::Subcommand, Debug)]
enum CmdKind {
    /// fork device list and serve if serve enabled
    Serve {
        /// port to serve to over quicc
        port: u16,

        /// certificate chain
        chain: PathBuf,

        /// private key
        privkey: PathBuf,
    },

    /// send device list to remote host on port and host
    Connect {
        /// list of devices to blktrace
        device: Vec<String>,

        /// remote port
        port: u16,

        /// remote host
        host: String,

        /// public key
        pubkey: PathBuf,

        /// server name as in server cert
        sname: String,
    },
}

const FRAGMENT_SIZE: usize = size_of::<blktrace::blk_io_trace>();

async fn process_input(input: &mut dyn Read) -> Result<(), Box<dyn Error>> {
    let mut buffer: [u8; FRAGMENT_SIZE] = [0; FRAGMENT_SIZE];
    while let Ok(()) = input.read_exact(&mut buffer) {
        let trace: blktrace::blk_io_trace = unsafe { std::mem::transmute(buffer) };
        let mut str_vec = Vec::<u8>::with_capacity(trace.pdu_len.into());
        io::copy(&mut input.take(trace.pdu_len.into()), &mut str_vec)?;
        let str: String = String::from(str::from_utf8(&str_vec)?);
        describe_histogram!(
            "iowatcherng-exporter.packet_time",
            "Histogram of packet processing time by main loop"
        );
        let now = Instant::now();
        buffer = [0; FRAGMENT_SIZE];
        match on_pkt(trace, &str) {
            Ok(()) => {
                let elapsed = now.elapsed();
                histogram!("iowatcherng-exporter.packet_time", elapsed, "ok" => "true")
            },
            Err(err) => {
                let elapsed = now.elapsed();
                histogram!("iowatcherng-exporter.packet_time", elapsed, "ok" => "false", "message" => format!("{}", err))
            },
        }
    }
    Ok(())
}

fn on_pkt(trace: blktrace::blk_io_trace, _str: &str) -> Result<(), Box<std::io::Error>> {
    if (trace.magic & 0xffffff00) != blktrace::BLK_IO_TRACE_MAGIC {
        eprintln!("Bad pkt magic");
        Err(Box::new(std::io::Error::new(
            io::ErrorKind::Unsupported,
            format!("cannot address packets with magic value {}", trace.magic),
        )))
    } else if (trace.action & !blktrace::blktrace_notify___BLK_TN_CGROUP) == blktrace::blktrace_notify___BLK_TN_MESSAGE
    {
        println!("NOTIFY");
        match trace.action & !blktrace::blktrace_notify___BLK_TN_CGROUP {
            blktrace::blktrace_notify___BLK_TN_PROCESS => println!("PROCESS"),
            blktrace::blktrace_notify___BLK_TN_TIMESTAMP => println!("TS"),
            blktrace::blktrace_notify___BLK_TN_MESSAGE => println!("MS"),
            _ => println!("Unk NOTIFY"),
        }
        Ok(())
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
        Ok(())
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
        Ok(())
    }
}

struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

pub fn read_certs_from_file(
    chain: &PathBuf,
    privkey: &PathBuf,
) -> Result<(Vec<rustls::Certificate>, rustls::PrivateKey), Box<dyn Error>> {
    let mut cert_chain_reader = BufReader::new(File::open(chain)?);
    let certs = rustls_pemfile::certs(&mut cert_chain_reader)?
        .into_iter()
        .map(rustls::Certificate)
        .collect();

    let mut key_reader = BufReader::new(File::open(privkey)?);
    // if the file starts with "BEGIN RSA PRIVATE KEY"
    // let mut keys = rustls_pemfile::rsa_private_keys(&mut key_reader)?;
    // if the file starts with "BEGIN PRIVATE KEY"
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)?;

    assert_eq!(keys.len(), 1);
    let key = rustls::PrivateKey(keys.remove(0));

    Ok((certs, key))
}

struct WorkerAdaptor {
    stealer: Stealer<Vec<u8>>,
}

impl Iterator for WorkerAdaptor {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Vec<u8>> {
        let item = self.stealer.steal();
        item.success()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    PrometheusBuilder::new()
        .with_http_listener(SocketAddr::from_str("::9975").unwrap())
        .install()
        .expect("able to install Prometheus recoder/exporter");

    match &args.command {
        CmdKind::Serve { port, chain, privkey } => {
            let (certs, privkey) = read_certs_from_file(chain, privkey)?;
            let mut buffer: [u8; FRAGMENT_SIZE] = [0; FRAGMENT_SIZE];
            match Endpoint::server(
                ServerConfig::with_single_cert(certs, privkey)?,
                SocketAddr::from_str(format!("::{}", port).as_str())?,
            ) {
                Ok(endpoint) => {
                    if let Some(accept) = endpoint.accept().await {
                        let (connection, _) = accept.into_0rtt().expect("can use 0rtt");
                        if let Ok(mut recv_stream) = connection.accept_uni().await {
                            todo!("Handle stream")
                        }
                        Ok(())
                    } else {
                        panic!("Cannot accept endpoint");
                    }
                },
                Err(why) => panic!("cannot serve: {}", why),
            }
        },
        CmdKind::Connect {
            device,
            port,
            host,
            pubkey,
            sname,
        } => {
            let mut arguments = Vec::new();
            for dev in device {
                arguments.push("-d".to_string());
                arguments.push(dev.to_string());
            }
            arguments.push("-o".to_string());
            arguments.push("-".to_string());
            match Command::new("blktrace").args(arguments).spawn() {
                Err(why) => panic!("couldn't spawn blktrace: {}", why),
                Ok(output) => {
                    let mut buffer: [u8; FRAGMENT_SIZE] = [0; FRAGMENT_SIZE];
                    let bytes_read = output.
                    let trace: blktrace::blk_io_trace = unsafe { std::mem::transmute(buffer) };
                    let mut instdout = child.stdout.expect("stdout is opened at this time");
                    let mut buffer: [u8; FRAGMENT_SIZE] = [0; FRAGMENT_SIZE];
                    let saddr = SocketAddr::from_str(format!("{}:{}", &host, &port).as_str())?;
                    match Endpoint::client(saddr) {
                        Ok(endpoint) => {
                            let (connection, zrtt) = endpoint.connect(saddr, sname)?.into_0rtt().expect("0rtt abled");
                            connection.op
                        },
                        Err(why) => panic!("Cannot create connection: {}", why),
                    }
                },
            }
            Ok(())
        },
    }
}
