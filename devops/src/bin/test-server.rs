use anyhow::{bail, Result};
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rand::rngs::OsRng;
use rand::rngs::ReseedingRng;
use rand::Rng;
use rand_chacha::ChaCha20Core;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn handle_request(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>> {
    match req.uri().path() {
        "/" => handle_root().await,
        "/sleep" => handle_sleep().await,
        "/mix" => handle_mix().await,
        "/close" => handle_close().await,
        "/fail" => handle_fail().await,
        "/header_dump" => handle_header_dump(&req).await,
        "/random" => handle_random_status().await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))?),
    }
}

async fn handle_root() -> Result<Response<Full<Bytes>>> {
    Ok(Response::new(Full::new(Bytes::from("Hello, world!"))))
}

async fn handle_sleep() -> Result<Response<Full<Bytes>>> {
    let mut reseeding_rng = ReseedingRng::<ChaCha20Core, _>::new(0, OsRng).unwrap();

    let delay_ms = reseeding_rng.random_range(100..=1000);

    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
    Ok(Response::new(Full::new(Bytes::from("Sleeping..."))))
}

async fn handle_fail() -> Result<Response<Full<Bytes>>> {
    bail!("fail");
}

async fn handle_close() -> Result<Response<Full<Bytes>>> {
    // Randomly close the connection
    let mut rng = rand::rng();
    let close_connection = rng.random_bool(0.3); // Close connection with 30% probability

    if close_connection {
        bail!("close connection");
    }

    // Return a normal response
    Ok(Response::new(Full::new(Bytes::from("OK"))))
}

async fn handle_random_status() -> Result<Response<Full<Bytes>>> {
    // Randomly change the status code
    let mut rng = rand::rng();
    let roll = rng.random_range(0..100);

    let status = match roll {
        0..50 => StatusCode::OK,                      // 50% 200 OK
        50..70 => StatusCode::MOVED_PERMANENTLY,      // 20% 301
        70..90 => StatusCode::BAD_REQUEST,            // 20% 400
        90..100 => StatusCode::INTERNAL_SERVER_ERROR, // 10% 500
        _ => StatusCode::OK,
    };

    Ok(Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(format!("Status: {}", status))))?)
}

async fn handle_mix() -> Result<Response<Full<Bytes>>> {
    let mut reseeding_rng = ReseedingRng::<ChaCha20Core, _>::new(0, OsRng).unwrap();

    let close_connection = reseeding_rng.random_bool(0.03); // 3% probability to close the connection
    if close_connection {
        bail!("close connection");
    }

    let roll = reseeding_rng.random_range(0..100);

    let status = match roll {
        0..50 => StatusCode::OK,                      // 50% 200 OK
        50..70 => StatusCode::MOVED_PERMANENTLY,      // 20% 301
        70..90 => StatusCode::BAD_REQUEST,            // 20% 400
        90..100 => StatusCode::INTERNAL_SERVER_ERROR, // 10% 500
        _ => StatusCode::OK,
    };

    let mut reseeding_rng = ReseedingRng::<ChaCha20Core, _>::new(0, OsRng).unwrap();
    let delay_ms = reseeding_rng.random_range(100..=1000);
    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;

    Ok(Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(format!("Status: {}", status))))?)
}

async fn handle_header_dump(req: &Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>> {
    let headers = req.headers();
    let mut header_string = String::new();
    for (key, value) in headers {
        header_string.push_str(&format!("{}: {}\n", key, value.to_str().unwrap()));
    }
    println!("{}", header_string);
    Ok(Response::new(Full::new(Bytes::from("OK"))))
}
