use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

use rand::Rng;

use crate::{
    dns::{
        cache::{insert_domain_ip_map, try_get_ip_from_cache},
        consts::{DNS_SERVER, DNS_TIMEOUT},
        protocol::{DnsResponse, EntireRecord, QRecordType, build_dns_query},
    },
    io_ext::{read::AsyncReader, write::AsyncBufWriter},
    result::{ErrorType, Result},
    select,
    web::conn::new_udp_conn,
};

pub mod cache;
pub(crate) mod consts;
mod protocol;

pub async fn dns_parse(domain: &str, port: u16) -> Result<SocketAddr> {
    if let Ok(addr) = domain.parse::<Ipv4Addr>() {
        return Ok(SocketAddr::V4(SocketAddrV4::new(addr, port)));
    }

    if let Some(ip) = try_get_ip_from_cache(domain).await {
        log::info!("find ip for {} from cache", domain);
        return Ok(SocketAddr::new(ip, port));
    }

    let ips = v4_v6_ip_lookup(domain).await?;
    if !ips.is_empty() {
        let ip = ips[0];
        insert_domain_ip_map(domain.into(), ip).await;
        return Ok(SocketAddr::new(ip, port));
    }

    Err(ErrorType::DnsParseFailed(format!("cannot find dns for {}", domain)).into())
}

async fn v4_v6_ip_lookup(domain: &str) -> Result<Vec<IpAddr>> {
    select! {
        Ok(ips) = ip_lookup(domain, QRecordType::A) => {
            log::debug!("find ipv4");
            return Ok(ips);
        },
        Ok(ips) = ip_lookup(domain, QRecordType::AAAA) => {
            log::debug!("find ipv6");
            return Ok(ips);
        },
    };

    Err(ErrorType::DnsParseFailed(format!("dns parse failed for {}", domain)).into())
}

async fn ip_lookup(domain: &str, record_type: QRecordType) -> Result<Vec<IpAddr>> {
    let id = rand::thread_rng().gen_range(0..u16::MAX);
    let query_body = build_dns_query(domain, id, record_type)?;
    let udp_conn = new_udp_conn(DNS_SERVER, DNS_TIMEOUT.into())?;

    let writer = AsyncBufWriter::from(udp_conn.clone());
    writer.lock().await.send(&query_body).await?;

    let mut reader = AsyncReader::from(udp_conn.clone());
    let resp_body = reader.read_once().await?;
    let dns_response = DnsResponse::deserialize(&resp_body, id)?;

    Ok(dns_response
        .records
        .iter()
        .filter_map(EntireRecord::get_ip_addr)
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use crate::{
        dns::{dns_parse, ip_lookup, protocol::QRecordType},
        err_log,
        result::Result,
        sync::wait_group::{WaitGroup, WaitGroupGuard},
        variable_log,
    };

    #[rt_entry::test(log_level = "debug")]
    async fn test_ip_lookup() -> Result<()> {
        let domain = "ark.cn-beijing.volces.com";
        let ips = err_log!(ip_lookup(domain, QRecordType::A).await, "dns parse failed")?;
        println!("find ips: {:?}", ips);

        Ok(())
    }

    #[rt_entry::test(log_level = "info")]
    async fn test_dns_lookup() -> Result<()> {
        let domain = "ark.cn-beijing.volces.com";
        let addr = dns_parse(domain, 80).await?;
        println!("find addr: {:?}", addr);
        Ok(())
    }

    #[rt_entry::test(log_level = "info")]
    async fn test_dns_lookup_2() -> Result<()> {
        let wg = Rc::new(WaitGroup::new());

        let lookup = async |domain: &str, _guard: WaitGroupGuard<'_>| {
            let _ = variable_log!(info @ dns_parse(domain, 80).await);
        };

        for domain in [
            "localhost",
            "ark.cn-beijing.volces.com",
            "www.zhihu.com",
            "www.jianshu.com",
            "www.bilibili.com",
        ] {
            spawn!(lookup(domain, wg.add()));
        }

        wg.wait().await;
        log::info!("all domain dns parsed");
        Ok(())
    }
}
