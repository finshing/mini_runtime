use std::time;

use crate::{config, sleep};

/// 非一次性超时
pub struct Timeout {
    end_at: time::Instant,
    timeout: time::Duration,
}

impl Timeout {
    pub fn new(timeout: time::Duration) -> Self {
        Self {
            end_at: time::Instant::now() + timeout,
            timeout,
        }
    }

    pub async fn timeout(&self) {
        let delay = self.end_at - time::Instant::now();
        if delay <= time::Duration::from_millis(0) {
            return;
        }

        sleep(delay).await;
    }
}

// clone会重新计算end_at
impl Clone for Timeout {
    fn clone(&self) -> Self {
        Self::new(self.timeout)
    }
}

pub struct ConnTimeout {
    // 完整链接最大等待时长
    _timeout: Timeout,
    // 单次读超时
    _read_timeout: Option<Timeout>,
    // 单次写超时
    _write_timeout: Option<Timeout>,
}

impl ConnTimeout {
    pub fn new(timeout: Option<time::Duration>) -> Self {
        Self {
            _timeout: Timeout::new(timeout.unwrap_or(config::DEFAULT_CONN_TIMEOUT)),
            _read_timeout: None,
            _write_timeout: None,
        }
    }

    #[allow(unused)]
    pub fn update_timeout(&mut self, timeout: time::Duration) -> &mut Self {
        self._timeout = Timeout::new(timeout);
        self
    }

    #[allow(unused)]
    pub fn set_read_timeout(&mut self, timeout: time::Duration) -> &mut Self {
        self._read_timeout.replace(Timeout::new(timeout));
        self
    }

    #[allow(unused)]
    pub fn set_write_timeout(&mut self, timeout: time::Duration) -> &mut Self {
        self._write_timeout.replace(Timeout::new(timeout));
        self
    }

    #[inline]
    pub(crate) async fn timeout(&self) {
        self._timeout.timeout().await
    }

    #[inline]
    pub(crate) async fn read_timeout(&self) {
        self._read_timeout
            .as_ref()
            .map_or(Timeout::new(config::DEFAULT_CONN_TIMEOUT), Clone::clone)
            .timeout()
            .await
    }

    #[inline]
    pub(crate) async fn write_timeout(&self) {
        self._write_timeout
            .as_ref()
            .map_or(Timeout::new(config::DEFAULT_CONN_TIMEOUT), Clone::clone)
            .timeout()
            .await
    }
}

impl Clone for ConnTimeout {
    fn clone(&self) -> Self {
        Self {
            _timeout: self._timeout.clone(),
            _read_timeout: self._read_timeout.clone(),
            _write_timeout: self._write_timeout.clone(),
        }
    }
}
