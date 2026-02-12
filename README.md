Rust单线程异步运行时
思路说明：https://zhuanlan.zhihu.com/p/1996615153593103605

支持功能：
1. 异步休眠
2. 通过属性宏启动可执行任务或者单元测试
3. TCP/UDP请求处理的异步化，并支持BufRead和BufWrite
4. 同步能力：异步锁、信号量、WaitGroup
5. 应用层面的多路复用（类似golang的select），并支持TCP/UDP的超时处理
6. 简单的HTTP协议、DNS解析等