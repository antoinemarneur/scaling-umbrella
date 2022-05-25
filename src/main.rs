use axum::{
    body::Bytes,
    routing::{get, post},
    Router,
    extract::{Path, BodyStream},
    BoxError,
};
use hyper::StatusCode;
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufWriter},
};
use tokio_util::io::StreamReader;
use futures::{Stream, TryStreamExt};
use std::io;

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/file/:file_name", get(get_file).post(post_file));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_file(Path(file_name): Path<String>) -> Result<String, (StatusCode, String)> {
    let mut file = match File::open(file_name).await {
        Ok(file) => file,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    };

    let mut contents = vec![];
    file.read_to_end(&mut contents).await.unwrap();

    let string_contents = String::from_utf8(contents).map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()));

    Ok(format!("{:?}", string_contents))
}

async fn post_file(Path(file_name): Path<String>, body: BodyStream) -> Result<(), (StatusCode, String)> {
    stream_to_file(&file_name, body)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

async fn stream_to_file<S, E>(path: &str, stream: S) -> Result<(), io::Error>
where
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    let io_error_stream = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
    let body_reader = StreamReader::new(io_error_stream);
    futures::pin_mut!(body_reader);

    let mut file = BufWriter::new(File::create(path).await?);
    tokio::io::copy(&mut body_reader, &mut file).await?;

    Ok(())
}