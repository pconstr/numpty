use futures::channel::oneshot;
use tokio::time::Duration;

pub struct Reply {
    pub lines: Vec<avt::Line>,
    pub error: Option<String>,
}

pub struct Req {
    pub wait_first: Duration,
    pub wait_more: Duration,
    pub reply: oneshot::Sender<Reply>,
}
