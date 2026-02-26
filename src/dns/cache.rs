use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Display,
    fs,
    io::{Read, Write},
    net::IpAddr,
    rc::Rc,
    str::FromStr,
    sync::atomic::{AtomicBool, Ordering},
    time,
};

use lazy_static::lazy_static;

use crate::{
    UPSafeCell,
    dns::consts::{DNS_CACHE_FILE, DNS_CACHE_TTL},
    err_log,
    result::Result,
    runtime::register_rt_finish_cb,
    select,
    signal::StopWaker,
    sleep, spawn,
    sync::mutex::AsyncMutex,
    variable_log,
};

static OPEN_DNS_CACHE_REFRESH: AtomicBool = AtomicBool::new(false);

// 开启dns缓存
pub fn open_dns_cache_refresh() {
    OPEN_DNS_CACHE_REFRESH.store(true, Ordering::Relaxed);
}

lazy_static! {
    static ref DNS_CACHER: UPSafeCell<Rc<AsyncMutex<DNSCache>>> = {
        let cacher = Rc::new(AsyncMutex::new(DNSCache::new().unwrap()));
        if OPEN_DNS_CACHE_REFRESH.load(Ordering::Relaxed) {
            // 在要求缓存开启的情况下异步执行缓存保存操作
            spawn!(periodic_dump(cacher.clone()));
        }
        register_rt_finish_cb(Box::new(cache_dump));
        UPSafeCell::new(cacher)
    };
}

pub async fn try_get_ip_from_cache(domain: &str) -> Option<IpAddr> {
    let cacher = DNS_CACHER.exclusive_access().clone();
    cacher.lock().await.try_get_ip(domain)
}

pub async fn insert_domain_ip_map(domain: Cow<'_, str>, ip: IpAddr) {
    let cacher = DNS_CACHER.exclusive_access().clone();
    cacher.lock().await.insert_new_ip(domain, ip);
}

// 运行时结束时进行缓存
fn cache_dump() {
    unsafe {
        let _ = variable_log!(info @ DNS_CACHER.exclusive_access().get_mut().dump(), "[dns cache dump]");
    }
}

// 周期性的存储dns映射
async fn periodic_dump(cache: Rc<AsyncMutex<DNSCache>>) {
    let stop_waker = StopWaker::default();
    let dur = time::Duration::from_secs(30);
    loop {
        select! {
            _ = stop_waker.wait() => {
                log::info!("dns stopping");
                return
            },
            _ = sleep(dur) => {
                log::info!("dump dns cache when loop");
                let _ = err_log!(cache.lock().await.dump());
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct DNSCache {
    domain_ip_map: HashMap<String, CachedItem>,
}

impl DNSCache {
    pub fn new() -> Result<Self> {
        let mut cache = Self::default();
        cache.load()?;

        Ok(cache)
    }

    pub fn try_get_ip(&mut self, domain: &str) -> Option<IpAddr> {
        if let Some(item) = self.domain_ip_map.get(domain)
            && !item.expired()
        {
            return Some(item.ip);
        }
        None
    }

    pub fn insert_new_ip(&mut self, dns: Cow<'_, str>, ip: IpAddr) {
        let item = CachedItem::new_with_ttl(dns, ip);
        self.domain_ip_map.insert(item.domain.clone(), item);
    }

    // 开启时从本地缓存文件中读取已缓存的映射关系
    fn load(&mut self) -> Result<()> {
        let mut file = match fs::OpenOptions::new().read(true).open(DNS_CACHE_FILE) {
            Ok(file) => Ok(file),
            Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound) => return Ok(()),
            Err(e) => Err(e),
        }?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let mut domain_ip_map = HashMap::new();
        for item_str in content.split('\n') {
            if let Ok(item) = CachedItem::from_str(item_str)
                && !item.expired()
            {
                domain_ip_map.insert(item.domain.clone(), item);
            }
        }
        self.domain_ip_map = domain_ip_map;
        println!("map: {:?}", self.domain_ip_map);

        Ok(())
    }

    fn dump(&self) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(DNS_CACHE_FILE)?;
        let content = self
            .domain_ip_map
            .values()
            .filter(|&item| !item.expired())
            .map(|item| format!("{}", item))
            .collect::<Vec<_>>()
            .join("\n");

        file.write_all(content.as_bytes())?;
        Ok(())
    }
}

impl Drop for DNSCache {
    fn drop(&mut self) {
        let _ = variable_log!(warn @ self.dump(), "[dns cache drop]");
    }
}

#[derive(Debug)]
pub(crate) struct CachedItem {
    domain: String,
    ip: IpAddr,
    // 支持ttl为空时的永久有效（如localhost的自定义）
    ttl: Option<i64>,
}

impl CachedItem {
    fn new_with_ttl(doamin: Cow<'_, str>, ip: IpAddr) -> Self {
        Self::new(
            doamin,
            ip,
            Some((chrono::Local::now() + DNS_CACHE_TTL).timestamp()),
        )
    }

    fn new(doamin: Cow<'_, str>, ip: IpAddr, ttl: Option<i64>) -> Self {
        Self {
            domain: doamin.into_owned(),
            ip,
            ttl,
        }
    }

    fn expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            return ttl < chrono::Local::now().timestamp();
        }

        false
    }

    fn from_str(value: impl AsRef<str>) -> Result<Self> {
        let mut splitter = value.as_ref().splitn(3, ' ');
        let domain = splitter.next().ok_or("domain not exist")?.into();
        let ip = IpAddr::from_str(splitter.next().ok_or("ip not exist")?)?;
        let ttl = splitter.next().map(|s| s.parse::<i64>()).transpose()?;

        Ok(Self::new(domain, ip, ttl))
    }
}

impl Display for CachedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.domain, self.ip).and_then(|_| {
            if let Some(ttl) = self.ttl {
                write!(f, " {}", ttl)
            } else {
                Ok(())
            }
        })
    }
}
