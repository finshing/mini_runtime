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
