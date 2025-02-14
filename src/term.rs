use tokio::sync::mpsc;
use tokio::task::JoinError;
use tokio::time::{sleep, Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::protocol::{Reply, Req};

pub async fn run_term(
    cols: usize,
    rows: usize,
    mut output_rx: mpsc::Receiver<Vec<u8>>,
    mut req_rx: mpsc::Receiver<Req>,
    token: CancellationToken,
) -> Result<(), JoinError> {
    tokio::spawn(async move {
        let mut maybe_waiting: Option<Req> = None;
        let mut req_until = Instant::now() + Duration::from_millis(9999999999);

        let mut closed_output = false;

        let mut vt = avt::Vt::builder().size(cols, rows).build();
        let error: Option<String> = None;

        let (_, mut never_rx) = mpsc::channel(1);

        loop {
            let now = Instant::now();
            let wait = req_until - now;

            tokio::select! {

                maybe_out = if closed_output {never_rx.recv()} else {output_rx.recv()} => {
                    match maybe_out {
                        Some(data) => {
                            vt.feed_str(&String::from_utf8_lossy(&data).to_string());

                            // got output, unsettling, reset wait
                            match &maybe_waiting {
                                Some(waiting) => {
                                    req_until = now + waiting.wait_more;
                                }
                                None => {}
                            }
                        }
                        None => {
                            closed_output = true;
                            match maybe_waiting.take() {
                                Some(waiting) => {
                                    let lines = vt.view().to_vec();
                                    let answer = Reply{lines: lines, error: error.clone()};
                                    // ignore failure, keep going until cancelled
                                    _ = waiting.reply.send(answer);
                                    req_until = Instant::now() + Duration::from_millis(9999999999);
                                    maybe_waiting = None
                                }
                                None => {}
                            }
                        }
                    }
                }
                maybe_req = req_rx.recv() => {
                    match maybe_req {
                        Some(req) => {
                            let now = Instant::now();
                            // if there was another one it will be cancelled
                            req_until = now + req.wait_first;
                            maybe_waiting = Some(req);
                        }
                        None => {
                            // channel has closed
                            break;
                        }
                    }
                }

                _ = token.cancelled() => {
                    break;
                }

                _ = sleep(wait) =>{
                    // settled
                    match maybe_waiting.take() {
                        Some(waiting) => {
                            let lines = vt.view().to_vec();
                            let answer = Reply{lines: lines, error: error.clone()};
                            // ignore failure, keep going until cancelled
                            _ = waiting.reply.send(answer);
                            req_until = Instant::now() + Duration::from_millis(9999999999);
                            maybe_waiting = None
                        }
                        None => {}
                    }
                }
            }
        }
    })
    .await
}
