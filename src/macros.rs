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

/// 这是一个应用层的多路复用宏，具体用法暂时可见于下方的单侧用例
#[macro_export]
macro_rules! select {
    ( $($pat:pat = $fut:expr => $cb:block $(|| $else_cb:block)? $(,)? )+ ) => {{
        $crate::select!(@ $($pat = $fut => $cb $(false, $else_cb)?,)+ )
    }};
    // 初始匹配处理，并开始记录站位符号
    ( @ $pat:pat = $fut:expr => $cb:block $($else:expr, $else_cb:block)?, $($branch:tt)* ) => {{
        $crate::select!(
            #
            ();
            (
                $pat = $fut, () => $cb $($else, $else_cb)?;
            )
            $($branch)*
        )
    }};
    // 进一步的匹配处理，$($s:tt)* 是站位符
    ( # ( $($s:tt)* ); ( $($ready:tt)+ ) $pat:pat = $fut:expr => $cb:block $($else:expr, $else_cb:block)?, $($branch:tt)* ) => {{
        $crate::select!(
            #
            ( $($s)* _ );
            (
                $($ready)+
                $pat = $fut, ( $($s)* _ ) => $cb $($else, $else_cb)?;
            )
            $($branch)*
        )
    }};
    ( # ( $($s:tt)* ); ( $($pat:pat = $fut:expr, ( $($place_holder:tt)* ) => $cb:block $($else:expr, $else_cb:block)?;)+ ) ) => {{
        // 如果不在这里将$async值固定下来，在poll_fn中会不断使用新的$async表达式
        // 最后增加一个`()`避免解包中的`..`的语法报错
        let mut future_with_results = ( $( $crate::helper::FutureExt::new_with_result_placeholder($fut), )+ ());
        #[allow(unused_assignments)]
        #[allow(irrefutable_let_patterns)]
        #[allow(unreachable_code)]
        #[allow(clippy::redundant_pattern_matching)]
        let output = $crate::helper::poll_fn(|mut cx| {
            log::debug!("start select macro");
            // 任务索引
            let mut idx = 0usize;
            // 已就绪（Poll::Ready），但非预期结果的任务数量。用于全部就绪但无需要结果场景下的兜底
            let mut unexpected = 0usize;
            $(
                // 解包
                let ( $($place_holder,)* future_with_result, ..) = &mut future_with_results;
                let pinned = unsafe {
                    std::pin::Pin::new_unchecked(&mut future_with_result.0)
                };
                match pinned.poll(&mut cx) {
                    std::task::Poll::Ready(result) => {
                        match result {
                            $crate::helper::FutureResult::Taken => {
                                unexpected += 1;
                            }
                            // 如果处于Pending状态，每个Future都会保留一个Waker的副本
                            // 这里的目的是只保留一个waker可以执行，避免事件就绪后的重复驱动
                            $crate::helper::FutureResult::Done(result) => {
                                let mut expect: Option<bool> = None;
                                // 判断是否符合分支要求
                                #[allow(unused)]
                                if matches!(&result, $pat) {
                                    expect.replace(true);
                                } $(else {
                                    expect.replace($else);
                                })?

                                log::debug!("select expect: {:?}", expect);
                                if let Some(expect) = expect {
                                    // 写入结果
                                    future_with_result.1.write(result);
                                    let task_attr = unsafe {
                                        $crate::TaskAttr::from_raw_data(cx.waker().data())
                                    };
                                    // 先取消其它waker
                                    task_attr.update_status($crate::TaskStatus::Cancelled);
                                    // 在重置当前waker的状态
                                    task_attr.set_status($crate::TaskStatus::Running);

                                    return std::task::Poll::Ready(Some((idx, expect)));
                                } else {
                                    unexpected += 1;
                                }
                            }
                        }
                    },
                    // 未就绪
                    _ => {}
                };
                idx += 1;
            )+

            log::debug!("all polled");
            // 全部任务就绪，但没有符合要求的结果
            if idx == unexpected {
                log::debug!("all brach unexpected");
                return std::task::Poll::Ready(None);
            }

            return std::task::Poll::Pending;
        }).await;

        log::debug!("select branch {:?}", output);
        if let Some((idx, ok)) = output {
            // 通过索引找到需要执行的代码块
            let mut count = 0usize;
            $(

                if count == idx {
                    if ok {
                        let ( $($place_holder,)* future_with_result, ..) = future_with_results;
                        #[allow(irrefutable_let_patterns)]
                        if let $pat = unsafe {future_with_result.1.assume_init()} $cb
                    } $(else $else_cb)?
                }
                count += 1;
            )+
        };
    }}
}

#[cfg(test)]
mod test {
    use std::time;

    use crate::{
        result::{ErrorType, Result},
        sleep,
    };

    #[rt_entry::test]
    async fn test_select_1() {
        let start_at = time::Instant::now();

        select!(
            _ = sleep(time::Duration::from_millis(200)) => {
                log::info!("in 200ms branch");
            },
            _ = sleep(time::Duration::from_millis(100)) => {
                log::info!("in 100ms branch");
            }
        );

        log::info!("total cost {}ms", start_at.elapsed().as_millis());
    }

    /*
     * 全部分支均不命中
     */
    #[rt_entry::test]
    async fn test_select_2() {
        async fn a(dur: time::Duration) -> Option<()> {
            sleep(dur).await;
            None
        }
        let start_at = time::Instant::now();

        select! {
            Some(_) = a(time::Duration::from_millis(200)) => {
                log::info!("in 200ms branch");
            },
            Some(_) = a(time::Duration::from_millis(100)) => {
                log::info!("in 100ms branch");
            }
        }

        log::info!("total cost {}ms", start_at.elapsed().as_millis());
    }

    /*
     * 命中模式匹配分支
     */
    #[rt_entry::test(log_level = "debug")]
    async fn test_select_3() {
        async fn a(dur: time::Duration, result: Option<()>) -> Option<()> {
            sleep(dur).await;
            result
        }
        let start_at = time::Instant::now();

        select! {
            Some(_) = a(time::Duration::from_millis(200), None) => {
                log::info!("in 200ms branch");
            },
            Some(_) = a(time::Duration::from_millis(150), Some(())) => {
                log::info!("in 150ms branch");
            },
            Some(_) = a(time::Duration::from_millis(100), None) => {
                log::info!("in 100ms branch: Some");
            }
        }

        log::info!("total cost {}ms", start_at.elapsed().as_millis());
    }

    /*
     * 多模式匹配命中
     */
    #[rt_entry::test(log_level = "debug")]
    async fn test_select_4() {
        async fn a(dur: time::Duration, result: Option<()>) -> Option<()> {
            sleep(dur).await;

            result
        }

        select! {
            Some(_) = a(time::Duration::from_secs(1), Some(())) => {
                log::info!("in 1 secs with Some");
            } || {
                log::info!("in 1 secs with None");
            },
            Some(_) = a(time::Duration::from_millis(500), Some(())) => {
                log::info!("in 500ms with Some");
            } || {
                log::info!("in 500ms with None")
            },
        }
    }

    #[rt_entry::test(log_level = "debug")]
    async fn test_select_5() {
        async fn a(dur: time::Duration, result: Result<()>) -> Result<()> {
            sleep(dur).await;
            result
        }

        select! {
            Err(e) = a(time::Duration::from_secs(1), Err(ErrorType::Timeout.into())) => {
                log::warn!("catch error: {:?}", e);
            } || {
                log::info!("branch 1 ok");
            },
            _ = a(time::Duration::from_millis(500), Ok(())) => {
                log::info!("branch 2 ok");
            }
        }
    }
}
