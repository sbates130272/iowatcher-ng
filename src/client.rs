use std::{
    error::Error,
    io::{self, Read},
    net::SocketAddr,
    path::PathBuf,
    str,
    str::FromStr,
    time::Instant,
};

use metrics::{describe_histogram, histogram};
use quinn::Endpoint;
use tokio::process::Command;

use crate::blktrace_api;

pub async fn connect(
    ca: &PathBuf,
    cert: &PathBuf,
    host: &String,
    port: &u16,
    devices: &Vec<String>,
) -> Result<(), Box<dyn Error>> {
    let mut arguments = Vec::new();
    for dev in devices {
        arguments.push("-d".to_string());
        arguments.push(dev.to_string());
    }
    arguments.push("-o".to_string());
    arguments.push("-".to_string());
    match Command::new("blktrace").args(arguments).spawn() {
        Err(why) => panic!("couldn't spawn blktrace: {}", why),
        Ok(child) => {
            let mut buffer: [u8; blktrace_api::FRAGMENT_SIZE] = [0; blktrace_api::FRAGMENT_SIZE];
            let trace: blktrace_api::blk_io_trace = unsafe { std::mem::transmute(buffer) };
            let mut stdout = child.stdout.expect("stdout is opened at this time");
            let mut buffer: [u8; blktrace_api::FRAGMENT_SIZE] = [0; blktrace_api::FRAGMENT_SIZE];
            let saddr = SocketAddr::from_str(format!("{}:{}", &host, &port).as_str())?;
            match Endpoint::client(saddr) {
                Ok(endpoint) => {
                    let (connection, _) = endpoint
                        .connect(saddr, &host)?
                        .into_0rtt()
                        .expect("0rtt abled");
                    let mut send_stream = connection.open_uni().await.unwrap();
                    tokio::io::copy(&mut stdout, &mut send_stream).await?;
                    send_stream.finish().await?;
                },
                Err(why) => panic!("Cannot create connection: {}", why),
            }
        },
    }
    Ok(())
}

async fn process_input(input: &mut dyn Read) -> Result<(), Box<dyn Error>> {
    let mut buffer: [u8; blktrace_api::FRAGMENT_SIZE] = [0; blktrace_api::FRAGMENT_SIZE];
    while let Ok(()) = input.read_exact(&mut buffer) {
        let trace: blktrace_api::blk_io_trace = unsafe { std::mem::transmute(buffer) };
        let mut str_vec = Vec::<u8>::with_capacity(trace.pdu_len.into());
        io::copy(&mut input.take(trace.pdu_len.into()), &mut str_vec)?;
        let str: String = String::from(str::from_utf8(&str_vec)?);
        describe_histogram!(
            "iowatcherng-exporter.packet_time",
            "Histogram of packet processing time by main loop"
        );
        let now = Instant::now();
        buffer = [0; blktrace_api::FRAGMENT_SIZE];
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

fn on_pkt(trace: blktrace_api::blk_io_trace, _str: &str) -> Result<(), Box<std::io::Error>> {
    if (trace.magic & 0xffffff00) != blktrace_api::BLK_IO_TRACE_MAGIC {
        eprintln!("Bad pkt magic");
        Err(Box::new(std::io::Error::new(
            io::ErrorKind::Unsupported,
            format!("cannot address packets with magic value {}", trace.magic),
        )))
    } else if (trace.action & !blktrace_api::blktrace_notify___BLK_TN_CGROUP)
        == blktrace_api::blktrace_notify___BLK_TN_MESSAGE
    {
        println!("NOTIFY");
        match trace.action & !blktrace_api::blktrace_notify___BLK_TN_CGROUP {
            blktrace_api::blktrace_notify___BLK_TN_PROCESS => println!("PROCESS"),
            blktrace_api::blktrace_notify___BLK_TN_TIMESTAMP => println!("TS"),
            blktrace_api::blktrace_notify___BLK_TN_MESSAGE => println!("MS"),
            _ => println!("Unk NOTIFY"),
        }
        Ok(())
    } else if (trace.action & blktrace_api::blk_tc_act(blktrace_api::blktrace_cat_BLK_TC_PC)) == 0 {
        println!("PC");
        let _w =
            (trace.action & blktrace_api::blk_tc_act(blktrace_api::blktrace_cat_BLK_TC_WRITE)) != 0;
        let act = (trace.action & 0xffff) & !blktrace_api::blktrace_act___BLK_TA_CGROUP;
        match act {
            blktrace_api::blktrace_act___BLK_TA_QUEUE => println!("TQ"),
            blktrace_api::blktrace_act___BLK_TA_GETRQ => println!("RQ"),
            blktrace_api::blktrace_act___BLK_TA_SLEEPRQ => println!("SPRQ"),
            blktrace_api::blktrace_act___BLK_TA_REQUEUE => println!("REQ"),
            blktrace_api::blktrace_act___BLK_TA_ISSUE => println!("ISSUE"),
            blktrace_api::blktrace_act___BLK_TA_COMPLETE => println!("COMPLETE"),
            blktrace_api::blktrace_act___BLK_TA_INSERT => println!("INSERT"),
            _ => println!("Unk PC"),
        }
        Ok(())
    } else {
        println!("CGROUP");
        let _w =
            (trace.action & blktrace_api::blk_tc_act(blktrace_api::blktrace_cat_BLK_TC_WRITE)) != 0;
        let act = (trace.action & 0xffff) & !blktrace_api::blktrace_act___BLK_TA_CGROUP;
        match act {
            blktrace_api::blktrace_act___BLK_TA_QUEUE => println!("TQ"),
            blktrace_api::blktrace_act___BLK_TA_INSERT => println!("RQ"),
            blktrace_api::blktrace_act___BLK_TA_BACKMERGE => println!("SPRQ"),
            blktrace_api::blktrace_act___BLK_TA_FRONTMERGE => println!("REQ"),
            blktrace_api::blktrace_act___BLK_TA_GETRQ => println!("ISSUE"),
            blktrace_api::blktrace_act___BLK_TA_SLEEPRQ => println!("COMPLETE"),
            blktrace_api::blktrace_act___BLK_TA_REQUEUE => println!("INSERT"),
            _ => println!("Unk CGROUP"),
        }
        Ok(())
    }
}
