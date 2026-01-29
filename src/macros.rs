#[macro_export]
macro_rules! spawn {
    ( $body:expr ) => {
        $crate::runtime::spawn($body);
    };
    ( $body:expr, $output_handle:expr ) => {
        $crate::runtime::spawn(async {
            let output = $body.await;
            $output_handle(output);
        })
    };
}

#[macro_export]
macro_rules! variable_log {
    ( $var:expr ) => {{
        $crate::variable_log!(info @ $var)
    }};
    ( $var:expr, $msg:expr ) => {{
        $crate::variable_log!(info @ $var, $msg)
    }};
    ( $level:ident @ $var:expr ) => {{
        $crate::variable_log!($level @ $var, "var")
    }};
    ( $level:ident @ $var:expr, $msg:expr ) => {{
        // 此处需要先计算出$var的值
        let v = $var;
        log::$level!("{} - {:?}", $msg, v);
        v
    }}
}

#[macro_export]
macro_rules! err_log {
    ( $result:expr ) => {{
        $crate::err_log!($result, "Err")
    }};
    ( $result:expr, $msg:expr ) => {{
        $crate::err_log!(warn @ $result, $msg)
    }};
    ( $level:ident @ $result:expr, $msg:expr ) => {{
        let result = $result;
        if let Err(ref e) = result {
            log::$level!("{} - {:?}", $msg, e);
        }
        result
    }};
}

/*
 * 创建web server
 * $handler是一个实现了Fn(Rc<AsyncMutex<Conn>>) -> Result<R, E>签名的函数
 * 这里会将$handler转换为符合Server要求的BoxedFutureWithError
 */
#[macro_export]
macro_rules! create_server {
    ($ip:expr, $port:expr, $handler:expr $(,)?) => {
        $crate::web::server::Server::new($ip, $port, |conn| {
            Box::pin(async { $handler(conn).await })
        })
    };
}
