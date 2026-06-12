// OHOS v0.1 主控端：KCP UDP 优化通道 stub。控制端 v0.1 用纯 TCP。v0.2 可补真。
#![allow(dead_code, unused_variables)]

use hbb_common::{anyhow::anyhow, tokio::net::UdpSocket, ResultType, Stream};
use std::sync::Arc;
use std::time::Duration;

pub struct KcpStream;

impl KcpStream {
    pub async fn accept<T>(_: T) -> ResultType<Self> {
        Err(anyhow!("OHOS v0.1: KcpStream::accept not implemented"))
    }

    pub async fn connect(_socket: Arc<UdpSocket>, _timeout: Duration) -> ResultType<(Self, Stream)> {
        Err(anyhow!("OHOS v0.1: KcpStream::connect not implemented"))
    }
}
