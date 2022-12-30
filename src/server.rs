use std::{error::Error, fs::File, io::BufReader, net::SocketAddr, path::PathBuf, str::FromStr};

use metrics_exporter_prometheus::PrometheusBuilder;
use quinn::{Endpoint, ServerConfig};

use crate::blktrace_api;

fn read_certs_from_file(
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

pub async fn serve(cert: &PathBuf, key: &PathBuf, port: &u16) -> Result<(), Box<dyn Error>> {
    PrometheusBuilder::new()
        .with_http_listener(SocketAddr::from_str("::9975").unwrap())
        .install()?;

    let (certs, privkey) = read_certs_from_file(cert, key)?;
    let mut buffer: [u8; blktrace_api::FRAGMENT_SIZE] = [0; blktrace_api::FRAGMENT_SIZE];
    match Endpoint::server(
        ServerConfig::with_single_cert(certs, privkey)?,
        SocketAddr::from_str(format!("::{}", port).as_str())?,
    ) {
        Ok(endpoint) => match endpoint.accept().await {
            Some(connect) => {
                let (connection, _) = connect.into_0rtt().expect("can use 0rtt");
                if let Ok(mut recv_stream) = connection.accept_uni().await {
                    todo!("Handle stream")
                }
                Ok(())
            },
            None => {
                panic!("Cannot accept endpoint");
            },
        },
        Err(why) => panic!("cannot serve: {}", why),
    }
}
